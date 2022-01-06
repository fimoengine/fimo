//! Implementation of the `ArrayString<N>` type.

/// A statically sized `String`.
///
/// The string contains valid `UTF-8` characters.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ArrayString<const LEN: usize> {
    data: [u8; LEN],
    length: usize,
}

/// An error resulting from [ArrayString::from_utf8].
#[derive(Debug)]
pub enum FromUtf8Error {
    /// An UTF-8 encoding error.
    Utf8Error(std::str::Utf8Error),
    /// A capacity error.
    CapacityError {
        /// Available capacity in bytes.
        available: usize,
        /// required capacity in bytes.
        required: usize,
    },
}

/// An error resulting from [ArrayString::push_str].
#[derive(Debug)]
pub struct PushStrError<'a> {
    str: &'a str,
    available_capacity: usize,
    required_capacity: usize,
}

/// An error resulting from [ArrayString::push].
#[derive(Debug)]
pub struct PushErr {
    ch: char,
    available_capacity: usize,
    required_capacity: usize,
}

impl<const LEN: usize> ArrayString<LEN> {
    /// Creates an empty `ArrayString`.
    pub fn new() -> Self {
        Self {
            data: [0; LEN],
            length: 0,
        }
    }

    /// Converts a slice of bytes to an `ArrayString`.
    ///
    /// If the slice contains valid UTF-8, and it fits in the allocated capacity,
    /// it is copied into the `ArrayString`.
    pub fn from_utf8(slice: &[u8]) -> Result<Self, FromUtf8Error> {
        if slice.len() > LEN {
            return Err(FromUtf8Error::CapacityError {
                available: LEN,
                required: slice.len(),
            });
        }

        match std::str::from_utf8(slice) {
            // SAFETY: The contents and the length of the slice are valid.
            Ok(_) => unsafe { Ok(Self::from_utf8_unchecked(slice)) },
            Err(e) => Err(FromUtf8Error::Utf8Error(e)),
        }
    }

    /// Converts a slice of bytes to an `ArrayString` without checking
    /// that the string contains valid UTF-8 or the required capacity.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it does not check whether the string
    /// contains valid UTF-8 or the available capacity suffices.
    pub unsafe fn from_utf8_unchecked(slice: &[u8]) -> Self {
        let mut string = Self::new();
        string.length = slice.len();
        string.data[..slice.len()].copy_from_slice(slice);

        string
    }

    /// Extracts a string slice containing the entire `ArrayString`.
    pub fn as_str(&self) -> &str {
        // SAFETY: The array contains valid utf-8 data.
        unsafe { std::str::from_utf8_unchecked(&self.data[..self.length]) }
    }

    /// Extracts a mutable string slice containing the entire `ArrayString`.
    pub fn as_mut_str(&mut self) -> &mut str {
        // SAFETY: The array contains valid utf-8 data.
        unsafe { std::str::from_utf8_unchecked_mut(&mut self.data[..self.length]) }
    }

    /// Tries to push a string at the end of the `ArrayString`.
    pub fn push_str<'a>(&mut self, string: &'a str) -> Result<(), PushStrError<'a>> {
        let slice = string.as_bytes();

        if self.length + slice.len() > LEN {
            return Err(PushStrError {
                str: string,
                available_capacity: LEN,
                required_capacity: self.length + slice.len(),
            });
        }

        let old_len = self.length;

        self.length += slice.len();
        self.data[old_len..self.length].copy_from_slice(slice);

        Ok(())
    }

    /// Extracts the capacity of the `ArrayString`.
    pub fn capacity(&self) -> usize {
        LEN
    }

    /// Tries to push a character at the end of the `ArrayString`.
    pub fn push(&mut self, ch: char) -> Result<(), PushErr> {
        if self.length + ch.len_utf8() > LEN {
            return Err(PushErr {
                ch,
                available_capacity: self.length,
                required_capacity: self.length + ch.len_utf8(),
            });
        }

        self.push_str(ch.encode_utf8(&mut [0; 4])).unwrap();
        Ok(())
    }

    /// Returns a byte slice of this `ArrayString`'s contents.
    pub fn as_bytes(&self) -> &[u8] {
        self.data.as_ref()
    }

    /// Extracts the length of the `ArrayString`.
    pub fn len(&self) -> usize {
        self.length
    }

    /// Returns `true` if this `ArrayString` has a length of zero, and `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears the content and the length of this `ArrayString`.
    pub fn clear(&mut self) {
        self.data = [0; LEN];
        self.length = 0;
    }
}

impl PushStrError<'_> {
    /// Extracts the string slice that caused the error.
    pub fn as_str(&self) -> &str {
        self.str
    }

    /// Extracts the available capacity of the `ArrayString` that caused the error.
    pub fn available_capacity(&self) -> usize {
        self.available_capacity
    }

    /// Extracts the required capacity tha `ArrayString` would have needed.
    pub fn required_capacity(&self) -> usize {
        self.required_capacity
    }
}

impl PushErr {
    /// Extracts the character that caused the error.
    pub fn as_char(&self) -> char {
        self.ch
    }

    /// Extracts the available capacity of the `ArrayString` that caused the error.
    pub fn available_capacity(&self) -> usize {
        self.available_capacity
    }

    /// Extracts the required capacity tha `ArrayString` would have needed.
    pub fn required_capacity(&self) -> usize {
        self.required_capacity
    }
}

impl std::fmt::Display for FromUtf8Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FromUtf8Error::Utf8Error(e) => std::fmt::Display::fmt(e, f),
            FromUtf8Error::CapacityError {
                available,
                required,
            } => std::fmt::Display::fmt(
                &format!(
                    "capacity exceeded: available {}, required {}",
                    available, required
                ),
                f,
            ),
        }
    }
}

impl std::fmt::Display for PushStrError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(
            &format!(
                "capacity exceeded: available {}, required {}, string {}",
                self.available_capacity, self.required_capacity, self.str
            ),
            f,
        )
    }
}

impl std::fmt::Display for PushErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(
            &format!(
                "capacity exceeded: available {}, required {}, char {}",
                self.available_capacity, self.required_capacity, self.ch
            ),
            f,
        )
    }
}

impl<const LEN: usize> Ord for ArrayString<LEN> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(self.as_str(), other.as_str())
    }
}

impl<const LEN: usize> PartialOrd for ArrayString<LEN> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        PartialOrd::partial_cmp(self.as_str(), other.as_str())
    }
}

impl<const LEN: usize> Eq for ArrayString<LEN> {}

impl<const LEN: usize> PartialEq for ArrayString<LEN> {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl<const LEN: usize> PartialEq<str> for ArrayString<LEN> {
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(self.as_str(), other)
    }
}

impl<'a, const LEN: usize> PartialEq<&'a str> for ArrayString<LEN> {
    fn eq(&self, other: &&'a str) -> bool {
        PartialEq::eq(self.as_str(), *other)
    }
}

impl<const LEN: usize> PartialEq<String> for ArrayString<LEN> {
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl<const LEN: usize> PartialEq<ArrayString<LEN>> for str {
    fn eq(&self, other: &ArrayString<LEN>) -> bool {
        PartialEq::eq(self, other.as_str())
    }
}

impl<'a, const LEN: usize> PartialEq<ArrayString<LEN>> for &'a str {
    fn eq(&self, other: &ArrayString<LEN>) -> bool {
        PartialEq::eq(*self, other.as_str())
    }
}

impl<const LEN: usize> PartialEq<ArrayString<LEN>> for String {
    fn eq(&self, other: &ArrayString<LEN>) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl<const LEN: usize> Default for ArrayString<LEN> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const LEN: usize> std::fmt::Display for ArrayString<LEN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_str(), f)
    }
}

impl<const LEN: usize> std::fmt::Debug for ArrayString<LEN> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<const LEN: usize> std::hash::Hash for ArrayString<LEN> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(self.as_str(), state)
    }
}

impl<const LEN: usize> std::ops::Deref for ArrayString<LEN> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<const LEN: usize> std::ops::DerefMut for ArrayString<LEN> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_str()
    }
}

impl<const LEN: usize> AsRef<str> for ArrayString<LEN> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<const LEN: usize> AsMut<str> for ArrayString<LEN> {
    fn as_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl<const LEN: usize> AsRef<[u8]> for ArrayString<LEN> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<'a, const LEN: usize> std::convert::TryFrom<&'a str> for ArrayString<LEN> {
    type Error = PushStrError<'a>;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let mut str = Self::new();
        str.push_str(value)?;

        Ok(str)
    }
}

impl<'a, const LEN: usize> std::convert::TryFrom<&'a mut str> for ArrayString<LEN> {
    type Error = PushStrError<'a>;

    fn try_from(value: &'a mut str) -> Result<Self, Self::Error> {
        let mut str = Self::new();
        str.push_str(value)?;

        Ok(str)
    }
}

impl<'a, const LEN: usize> std::convert::TryFrom<&'a String> for ArrayString<LEN> {
    type Error = PushStrError<'a>;

    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        let mut str = Self::new();
        str.push_str(value.as_str())?;

        Ok(str)
    }
}

impl<const LEN: usize> std::convert::TryFrom<char> for ArrayString<LEN> {
    type Error = PushErr;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        let mut str = Self::new();
        str.push(value)?;

        Ok(str)
    }
}
