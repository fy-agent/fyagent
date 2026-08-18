use std::ops::Deref;

pub struct Zeroizing<T: AsMut<[u8]>>(T);

impl Zeroizing<Vec<u8>> {
    pub fn new(value: Vec<u8>) -> Self {
        Self(value)
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl<T: AsMut<[u8]> + AsRef<[u8]>> Deref for Zeroizing<T> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl<T: AsMut<[u8]>> Drop for Zeroizing<T> {
    fn drop(&mut self) {
        for byte in self.0.as_mut() {
            unsafe {
                std::ptr::write_volatile(byte, 0);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Choice(u8);

impl From<Choice> for bool {
    fn from(choice: Choice) -> Self {
        choice.0 != 0
    }
}

pub trait ConstantTimeEq {
    fn ct_eq(&self, other: &Self) -> Choice;
}

impl ConstantTimeEq for [u8] {
    fn ct_eq(&self, other: &Self) -> Choice {
        if self.len() != other.len() {
            return Choice(0);
        }
        let mut acc = 0u8;
        for (left, right) in self.iter().zip(other.iter()) {
            acc |= left ^ right;
        }
        Choice(u8::from(acc == 0))
    }
}
