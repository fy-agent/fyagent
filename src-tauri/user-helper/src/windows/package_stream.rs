use std::{
    fs::File,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    os::windows::fs::FileExt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use windows::{core::HSTRING, Foundation::Uri};

use super::{
    native_file_identity, BridgeOperationId, DeploymentFailure, HelperErrorCode,
    NativeFileIdentity, NativeObjectKind,
};

const REQUEST_MAX_BYTES: usize = 16 * 1024;
const TRANSFER_BUFFER_BYTES: usize = 128 * 1024;
const WORKER_COUNT: usize = 4;
const HEADER_TIMEOUT: Duration = Duration::from_millis(500);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const ACCEPT_RETRY_DELAY: Duration = Duration::from_millis(5);

pub(super) struct PackageStreamServer {
    address: SocketAddr,
    request_path: String,
    stop: Arc<AtomicBool>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl PackageStreamServer {
    pub(super) fn start(
        package_file: &File,
        identity: NativeFileIdentity,
        operation_id: BridgeOperationId,
    ) -> Result<Self, DeploymentFailure> {
        let listener =
            TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|_| package_uri_failure())?;
        listener
            .set_nonblocking(true)
            .map_err(|_| package_uri_failure())?;
        let address = listener.local_addr().map_err(|_| package_uri_failure())?;
        if address.ip() != Ipv4Addr::LOCALHOST {
            return Err(package_uri_failure());
        }
        let request_path = format!(
            "/fyagent-package/{}/installer.msix",
            operation_id.directory_name()
        );
        let stop = Arc::new(AtomicBool::new(false));
        let mut workers: Vec<thread::JoinHandle<()>> = Vec::with_capacity(WORKER_COUNT);
        for index in 0..WORKER_COUNT {
            let worker_listener = listener.try_clone().map_err(|_| package_uri_failure())?;
            let worker_file = package_file
                .try_clone()
                .map_err(|_| package_uri_failure())?;
            let worker_path = request_path.clone();
            let worker_stop = stop.clone();
            let worker = match thread::Builder::new()
                .name(format!("fyagent-package-stream-{index}"))
                .spawn(move || {
                    serve_package_requests(
                        worker_listener,
                        worker_file,
                        identity,
                        worker_path,
                        worker_stop,
                    );
                }) {
                Ok(worker) => worker,
                Err(_) => {
                    stop.store(true, Ordering::Release);
                    for worker in workers {
                        let _ = worker.join();
                    }
                    return Err(package_uri_failure());
                }
            };
            workers.push(worker);
        }
        Ok(Self {
            address,
            request_path,
            stop,
            workers,
        })
    }

    pub(super) fn package_uri(&self) -> Result<Uri, DeploymentFailure> {
        let value = format!(
            "http://127.0.0.1:{}{}",
            self.address.port(),
            self.request_path
        );
        let uri = Uri::CreateUri(&HSTRING::from(value)).map_err(|_| package_uri_failure())?;
        let scheme = uri.SchemeName().map_err(|_| package_uri_failure())?;
        let host = uri.Host().map_err(|_| package_uri_failure())?;
        let query = uri.Query().map_err(|_| package_uri_failure())?;
        let fragment = uri.Fragment().map_err(|_| package_uri_failure())?;
        if scheme != "http" || host != "127.0.0.1" || !query.is_empty() || !fragment.is_empty() {
            return Err(package_uri_failure());
        }
        Ok(uri)
    }
}

impl Drop for PackageStreamServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        for _ in 0..self.workers.len() {
            let _ = TcpStream::connect_timeout(&self.address, Duration::from_millis(100));
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn package_uri_failure() -> DeploymentFailure {
    DeploymentFailure::Operation(HelperErrorCode::PackageUriInvalid)
}

fn serve_package_requests(
    listener: TcpListener,
    package_file: File,
    identity: NativeFileIdentity,
    request_path: String,
    stop: Arc<AtomicBool>,
) {
    let mut request_buffer = [0_u8; REQUEST_MAX_BYTES];
    let mut transfer_buffer = vec![0_u8; TRANSFER_BUFFER_BYTES];
    while !stop.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((mut stream, peer)) if peer.ip().is_loopback() => {
                if stream.set_write_timeout(Some(WRITE_TIMEOUT)).is_err() {
                    continue;
                }
                let _ = serve_package_request(
                    &mut stream,
                    &package_file,
                    identity,
                    &request_path,
                    &stop,
                    &mut request_buffer,
                    &mut transfer_buffer,
                );
            }
            Ok((stream, _)) => drop(stream),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_RETRY_DELAY);
            }
            Err(_) => break,
        }
    }
}

fn serve_package_request(
    stream: &mut TcpStream,
    package_file: &File,
    identity: NativeFileIdentity,
    request_path: &str,
    stop: &AtomicBool,
    request_buffer: &mut [u8; REQUEST_MAX_BYTES],
    transfer_buffer: &mut [u8],
) -> std::io::Result<()> {
    let request_length = match read_http_request(stream, request_buffer, stop)? {
        Some(length) => length,
        None => return write_http_error(stream, 400, "Bad Request"),
    };
    let Some((method, range)) = parse_package_request(
        &request_buffer[..request_length],
        request_path,
        identity.size,
    ) else {
        return write_http_error(stream, 404, "Not Found");
    };
    if native_file_identity(package_file, NativeObjectKind::RegularFile).ok() != Some(identity)
        || identity.size == 0
    {
        return write_http_error(stream, 503, "Service Unavailable");
    }
    let (status, start, end) = match range {
        PackageHttpRange::Full => (("200", "OK"), 0, identity.size - 1),
        PackageHttpRange::Bytes { start, end } => (("206", "Partial Content"), start, end),
        PackageHttpRange::Unsatisfiable => {
            write!(
                stream,
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                identity.size
            )?;
            return stream.flush();
        }
    };
    let content_length = end - start + 1;
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: application/msix\r\nAccept-Ranges: bytes\r\nContent-Length: {}\r\n",
        status.0, status.1, content_length
    )?;
    if status.0 == "206" {
        write!(
            stream,
            "Content-Range: bytes {}-{}/{}\r\n",
            start, end, identity.size
        )?;
    }
    write!(stream, "Connection: close\r\n\r\n")?;
    if method == PackageHttpMethod::Head {
        return stream.flush();
    }

    let mut offset = start;
    let mut remaining = content_length;
    while remaining > 0 && !stop.load(Ordering::Acquire) {
        let requested = usize::try_from(remaining.min(transfer_buffer.len() as u64))
            .expect("bounded package response chunk");
        let read = package_file.seek_read(&mut transfer_buffer[..requested], offset)?;
        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "pinned package ended during loopback transfer",
            ));
        }
        stream.write_all(&transfer_buffer[..read])?;
        let read = read as u64;
        offset += read;
        remaining -= read;
    }
    stream.flush()
}

fn read_http_request(
    stream: &mut TcpStream,
    request: &mut [u8; REQUEST_MAX_BYTES],
    stop: &AtomicBool,
) -> std::io::Result<Option<usize>> {
    let deadline = Instant::now() + HEADER_TIMEOUT;
    let mut length = 0;
    loop {
        if stop.load(Ordering::Acquire) {
            return Ok(None);
        }
        if request[..length]
            .windows(4)
            .any(|window| window == b"\r\n\r\n")
        {
            return Ok(Some(length));
        }
        if length == request.len() {
            return Ok(None);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        stream.set_read_timeout(Some(remaining))?;
        let read = stream.read(&mut request[length..])?;
        if read == 0 {
            return Ok(None);
        }
        length += read;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PackageHttpMethod {
    Get,
    Head,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PackageHttpRange {
    Full,
    Bytes { start: u64, end: u64 },
    Unsatisfiable,
}

fn parse_package_request(
    request: &[u8],
    expected_path: &str,
    package_size: u64,
) -> Option<(PackageHttpMethod, PackageHttpRange)> {
    let request = std::str::from_utf8(request).ok()?;
    let headers_end = request.find("\r\n\r\n")?;
    let mut lines = request[..headers_end].split("\r\n");
    let mut request_line = lines.next()?.split(' ');
    let method = match request_line.next()? {
        "GET" => PackageHttpMethod::Get,
        "HEAD" => PackageHttpMethod::Head,
        _ => return None,
    };
    if request_line.next()? != expected_path
        || !matches!(request_line.next()?, "HTTP/1.0" | "HTTP/1.1")
        || request_line.next().is_some()
    {
        return None;
    }

    let mut range = None;
    for line in lines {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("range") {
            if range.is_some() {
                return None;
            }
            range = Some(value.trim());
        } else if name.eq_ignore_ascii_case("transfer-encoding")
            || (name.eq_ignore_ascii_case("content-length") && value.trim() != "0")
        {
            return None;
        }
    }
    Some((
        method,
        range.map_or(PackageHttpRange::Full, |value| {
            parse_package_range(value, package_size)
        }),
    ))
}

fn parse_package_range(value: &str, size: u64) -> PackageHttpRange {
    let Some(range) = value.strip_prefix("bytes=") else {
        return PackageHttpRange::Unsatisfiable;
    };
    if size == 0 || range.contains(',') {
        return PackageHttpRange::Unsatisfiable;
    }
    let Some((start, end)) = range.split_once('-') else {
        return PackageHttpRange::Unsatisfiable;
    };
    if start.is_empty() {
        let Ok(suffix_length) = end.parse::<u64>() else {
            return PackageHttpRange::Unsatisfiable;
        };
        if suffix_length == 0 {
            return PackageHttpRange::Unsatisfiable;
        }
        return PackageHttpRange::Bytes {
            start: size.saturating_sub(suffix_length),
            end: size - 1,
        };
    }
    let Ok(start) = start.parse::<u64>() else {
        return PackageHttpRange::Unsatisfiable;
    };
    if start >= size {
        return PackageHttpRange::Unsatisfiable;
    }
    let end = if end.is_empty() {
        size - 1
    } else {
        let Ok(end) = end.parse::<u64>() else {
            return PackageHttpRange::Unsatisfiable;
        };
        end.min(size - 1)
    };
    if end < start {
        PackageHttpRange::Unsatisfiable
    } else {
        PackageHttpRange::Bytes { start, end }
    }
}

fn write_http_error(stream: &mut TcpStream, code: u16, reason: &str) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {code} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::{Shutdown, TcpStream},
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use fyagent_user_helper::BRIDGE_OPERATION_ID_BYTES;

    #[test]
    fn range_parser_accepts_one_bounded_range() {
        assert_eq!(
            parse_package_range("bytes=2-5", 10),
            PackageHttpRange::Bytes { start: 2, end: 5 }
        );
        assert_eq!(
            parse_package_range("bytes=8-", 10),
            PackageHttpRange::Bytes { start: 8, end: 9 }
        );
        assert_eq!(
            parse_package_range("bytes=-3", 10),
            PackageHttpRange::Bytes { start: 7, end: 9 }
        );
        for rejected in ["items=0-1", "bytes=10-", "bytes=5-4", "bytes=0-1,3-4"] {
            assert_eq!(
                parse_package_range(rejected, 10),
                PackageHttpRange::Unsatisfiable,
                "{rejected}"
            );
        }
    }

    #[test]
    fn trickle_clients_cannot_hold_workers_or_delay_drop() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fyagent-package-server-{}-{unique}.msix",
            std::process::id()
        ));
        let mut file = File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        file.write_all(b"0123456789").unwrap();
        file.flush().unwrap();
        let identity = native_file_identity(&file, NativeObjectKind::RegularFile).unwrap();
        let operation_id = BridgeOperationId::new([0xAB; BRIDGE_OPERATION_ID_BYTES])
            .expect("nonzero operation ID");
        let server = PackageStreamServer::start(&file, identity, operation_id)
            .unwrap_or_else(|_| panic!("package stream server must start for a local pinned file"));

        let mut tricklers = Vec::with_capacity(WORKER_COUNT);
        for _ in 0..WORKER_COUNT {
            let mut stream = TcpStream::connect(server.address).unwrap();
            tricklers.push(thread::spawn(move || {
                for _ in 0..40 {
                    if stream.write_all(b"G").is_err() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            }));
        }
        thread::sleep(HEADER_TIMEOUT + Duration::from_millis(100));

        let started = Instant::now();
        let ranged = package_server_request(
            server.address,
            &format!(
                "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nRange: bytes=2-5\r\n\r\n",
                server.request_path
            ),
        );
        assert!(started.elapsed() < HEADER_TIMEOUT);
        assert!(ranged.starts_with(b"HTTP/1.1 206 Partial Content\r\n"));
        assert!(ranged.ends_with(b"2345"));

        let denied = package_server_request(
            server.address,
            "GET /fyagent-package/not-the-capability/installer.msix HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        );
        assert!(denied.starts_with(b"HTTP/1.1 404 Not Found\r\n"));

        let drop_started = Instant::now();
        drop(server);
        assert!(drop_started.elapsed() < HEADER_TIMEOUT);
        for trickler in tricklers {
            trickler.join().unwrap();
        }
        drop(file);
        std::fs::remove_file(path).unwrap();
    }

    fn package_server_request(address: SocketAddr, request: &str) -> Vec<u8> {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        stream.shutdown(Shutdown::Write).unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        response
    }
}
