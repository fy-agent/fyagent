//! Bounded, namespace-aware manifest parsing. DTDs and external entities are
//! never accepted. Only the direct Identity of the single package root counts.
use quick_xml::{events::Event, name::ResolveResult, reader::NsReader};
use std::collections::HashSet;

use crate::codex_desktop::error::{InstallerError, InstallerErrorCode};

fn invalid() -> InstallerError {
    InstallerError::new(InstallerErrorCode::PackageParseFailed)
        .with_diagnostic_message("安装包 XML 清单无效或身份字段缺失，已拒绝安装")
}

fn valid_name(name: &[u8]) -> bool {
    // Manifest vocabulary uses ASCII QNames. Reject malformed names rather
    // than relying on the streaming reader to validate XML name productions.
    let parts = name.split(|byte| *byte == b':').collect::<Vec<_>>();
    parts.len() <= 2
        && parts.iter().all(|part| {
            part.first()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
                && part
                    .iter()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(byte))
        })
}

fn valid_xml_chars(text: &str) -> bool {
    text.chars().all(|ch| matches!(ch, '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}'))
}

pub(super) fn parse_identity(
    bytes: &[u8],
    bundle: bool,
) -> Result<(String, String), InstallerError> {
    let xml = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    if bytes.len() > 1024 * 1024 || !valid_xml_chars(xml) {
        return Err(invalid());
    }
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().check_comments = true;
    let mut depth = 0usize;
    let mut root_seen = false;
    let mut declaration_seen = false;
    let mut root_namespace = Vec::new();
    let mut identity = None;
    let mut events = 0usize;
    loop {
        events += 1;
        if events > 100_000 {
            return Err(invalid());
        }
        let event = reader.read_event().map_err(|_| invalid())?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                if depth >= 64 || !valid_name(element.name().as_ref()) {
                    return Err(invalid());
                }
                let (namespace, local) = reader.resolver().resolve_element(element.name());
                let namespace = match namespace {
                    ResolveResult::Bound(namespace) => namespace.as_ref().to_vec(),
                    ResolveResult::Unbound => Vec::new(),
                    ResolveResult::Unknown(_) => return Err(invalid()),
                };
                if depth == 0 {
                    if root_seen {
                        return Err(invalid());
                    }
                    root_seen = true;
                    let correct = if bundle {
                        local.as_ref() == b"Bundle"
                            && namespace == b"http://schemas.microsoft.com/appx/2013/bundle"
                    } else {
                        local.as_ref() == b"Package"
                            && [
                                b"http://schemas.microsoft.com/appx/manifest/foundation/windows10"
                                    .as_slice(),
                                b"http://schemas.microsoft.com/appx/2010/manifest".as_slice(),
                            ]
                            .contains(&namespace.as_slice())
                    };
                    if !correct {
                        return Err(invalid());
                    }
                    root_namespace = namespace.clone();
                }
                let is_identity = local.as_ref() == b"Identity";
                if is_identity && (depth != 1 || namespace != root_namespace || identity.is_some())
                {
                    return Err(invalid());
                }
                let mut name = None;
                let mut publisher = None;
                let mut expanded_attributes = HashSet::new();
                for (index, attribute) in element.attributes().enumerate() {
                    if index >= 256 {
                        return Err(invalid());
                    }
                    let attribute = attribute.map_err(|_| invalid())?;
                    if !valid_name(attribute.key.as_ref()) {
                        return Err(invalid());
                    }
                    if attribute.value.contains(&b'<') {
                        return Err(invalid());
                    }
                    let value = attribute
                        .decoded_and_normalized_value(
                            quick_xml::XmlVersion::Implicit1_0,
                            reader.decoder(),
                        )
                        .map_err(|_| invalid())?;
                    if !valid_xml_chars(&value) {
                        return Err(invalid());
                    }
                    if attribute.key.as_ref() == b"xmlns"
                        || attribute.key.as_ref().starts_with(b"xmlns:")
                    {
                        continue;
                    }
                    let (namespace, local) = reader.resolver().resolve_attribute(attribute.key);
                    let namespace = match namespace {
                        ResolveResult::Bound(namespace) => namespace.as_ref().to_vec(),
                        ResolveResult::Unbound => Vec::new(),
                        ResolveResult::Unknown(_) => return Err(invalid()),
                    };
                    if !expanded_attributes.insert((namespace.clone(), local.as_ref().to_vec())) {
                        return Err(invalid());
                    }
                    if is_identity && (local.as_ref() == b"Name" || local.as_ref() == b"Publisher")
                    {
                        if !namespace.is_empty() {
                            return Err(invalid());
                        }
                        let slot = if local.as_ref() == b"Name" {
                            &mut name
                        } else {
                            &mut publisher
                        };
                        if slot.replace(value.into_owned()).is_some() {
                            return Err(invalid());
                        }
                    }
                }
                if is_identity {
                    identity = Some((
                        name.filter(|value| !value.is_empty()).ok_or_else(invalid)?,
                        publisher
                            .filter(|value| !value.is_empty())
                            .ok_or_else(invalid)?,
                    ));
                }
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
            }
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or_else(invalid)?;
            }
            Event::Text(text) => {
                let text = text.xml10_content().map_err(|_| invalid())?;
                if text.contains("]]>") {
                    return Err(invalid());
                }
                if depth == 0
                    && !text
                        .chars()
                        .all(|ch| matches!(ch, ' ' | '\t' | '\r' | '\n'))
                {
                    return Err(invalid());
                }
            }
            Event::GeneralRef(reference) => {
                if depth == 0 {
                    return Err(invalid());
                }
                let text = reference.decode().map_err(|_| invalid())?;
                let encoded = format!("&{text};");
                let decoded = quick_xml::escape::unescape(&encoded).map_err(|_| invalid())?;
                if !valid_xml_chars(&decoded) {
                    return Err(invalid());
                }
            }
            Event::CData(_) if depth == 0 => return Err(invalid()),
            Event::Decl(declaration) => {
                if events != 1
                    || root_seen
                    || declaration_seen
                    || declaration.version().map_err(|_| invalid())?.as_ref() != b"1.0"
                {
                    return Err(invalid());
                }
                declaration_seen = true;
                let declaration_text =
                    std::str::from_utf8(declaration.as_ref()).map_err(|_| invalid())?;
                let attributes = quick_xml::events::BytesStart::from_content(declaration_text, 3);
                let mut previous_attribute = 0;
                for attribute in attributes.attributes() {
                    let attribute = attribute.map_err(|_| invalid())?;
                    let order = match attribute.key.as_ref() {
                        b"version" => 1,
                        b"encoding" => 2,
                        b"standalone" => 3,
                        _ => return Err(invalid()),
                    };
                    if order <= previous_attribute {
                        return Err(invalid());
                    }
                    previous_attribute = order;
                }
                if let Some(encoding) = declaration.encoding() {
                    if !encoding
                        .map_err(|_| invalid())?
                        .eq_ignore_ascii_case(b"UTF-8")
                    {
                        return Err(invalid());
                    }
                }
                if let Some(standalone) = declaration.standalone() {
                    if ![b"yes".as_slice(), b"no".as_slice()]
                        .contains(&standalone.map_err(|_| invalid())?.as_ref())
                    {
                        return Err(invalid());
                    }
                }
            }
            Event::DocType(_) | Event::PI(_) => return Err(invalid()),
            Event::Eof => {
                if depth != 0 || !root_seen {
                    return Err(invalid());
                }
                return identity.ok_or_else(invalid);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const NS: &str = "http://schemas.microsoft.com/appx/manifest/foundation/windows10";
    #[test]
    fn namespace_and_direct_identity_are_required() {
        let valid = format!(
            r#"<p:Package xmlns:p="{NS}"><p:Identity Name="OpenAI.Codex" Publisher="CN=test"/></p:Package>"#
        );
        assert_eq!(
            parse_identity(valid.as_bytes(), false).unwrap(),
            ("OpenAI.Codex".into(), "CN=test".into())
        );
        for xml in [
            valid.replace(NS, "urn:wrong"),
            valid.replace("p:Identity", "Identity"),
            valid.replace("Publisher=", "Missing="),
            valid.replace("Name=", "p:Name="),
            valid
                .replace("<p:Identity", "<Wrapper><p:Identity")
                .replace("/></p:Package>", "/></Wrapper></p:Package>"),
        ] {
            assert!(parse_identity(xml.as_bytes(), false).is_err(), "{xml}");
        }
    }
    #[test]
    fn xml_depth_event_and_byte_limits_reject() {
        let identity = r#"<Identity Name="OpenAI.Codex" Publisher="CN=test"/>"#;
        let prefix = format!(r#"<Package xmlns="{NS}">{identity}"#);
        let deep = format!(
            "{prefix}{}{}</Package>",
            "<A>".repeat(65),
            "</A>".repeat(65)
        );
        let many = format!("{prefix}{}</Package>", "<A/>".repeat(100_000));
        let oversized = format!("{prefix}{}</Package>", " ".repeat(1024 * 1024));
        let many_attributes = format!(
            "{prefix}<A {}/></Package>",
            (0..257).map(|i| format!("a{i}=\"x\" ")).collect::<String>()
        );
        for xml in [deep, many, oversized, many_attributes] {
            assert!(parse_identity(xml.as_bytes(), false).is_err());
        }
    }
    #[test]
    fn malformed_duplicate_dtd_entity_and_trailing_xml_are_rejected() {
        let good = format!(
            r#"<Package xmlns="{NS}"><Identity Name="OpenAI.Codex" Publisher="CN=test"/></Package>"#
        );
        for xml in [
            good.replace("Name=", "Name=\"duplicate\" Name="),
            good.replace("</Package>", ""),
            good.replace("</Package>", "</Wrong>"),
            format!("{good}{good}"),
            format!("{good}junk"),
            format!(r#"<?xml version="1.0" standalone="yes" encoding="UTF-8"?>{good}"#),
            format!(r#" <?xml version="1.0"?>{good}"#),
            format!("<!DOCTYPE Package [<!ENTITY ext SYSTEM 'file:///secret'>]>{good}"),
            good.replace("CN=test", "&undefined;"),
            good.replace("<Identity", "<Identity Name=\"duplicate\""),
            good.replace("/>", "/><Identity Name=\"other\" Publisher=\"other\"/>"),
            good.replace("CN=test", "CN=\u{0}"),
            good.replace("</Package>", "&undefined;</Package>"),
            good.replace("<Identity", "<unbound:Identity"),
            good.replace("</Package>", "<?pi x?></Package>"),
            good.replace("<Identity", "<1Identity"),
            good.replace("CN=test", "&#0;"),
            good.replace("CN=test", "CN=<bad"),
            good.replace("</Package>", "]]></Package>"),
            good.replace("</Package>", "&#xFFFE;</Package>"),
        ] {
            assert!(parse_identity(xml.as_bytes(), false).is_err(), "{xml}");
        }
        assert!(parse_identity(&[0xff], false).is_err());
        assert!(parse_identity(good.as_bytes(), true).is_err());
    }
}
