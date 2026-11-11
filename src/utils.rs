use base64::engine;

pub(crate) const BASE64: engine::GeneralPurpose = engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    engine::GeneralPurposeConfig::new()
        .with_decode_allow_trailing_bits(true)
        .with_decode_padding_mode(engine::DecodePaddingMode::Indifferent)
        .with_encode_padding(true),
);

const fn is_tchar(c: u8) -> bool {
    // See tchar values list in https://tools.ietf.org/html/rfc7230#section-3.2.6
    matches!(
        c,
        b'!' | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~'
    ) || c.is_ascii_alphanumeric()
}

pub(crate) const fn is_allowed_start_token_char(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'*'
}

pub(crate) const fn is_allowed_inner_token_char(c: u8) -> bool {
    is_tchar(c) || c == b':' || c == b'/'
}

pub(crate) const fn is_allowed_start_key_char(c: u8) -> bool {
    c.is_ascii_lowercase() || c == b'*'
}

pub(crate) const fn is_allowed_inner_key_char(c: u8) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'_' | b'-' | b'*' | b'.')
}

pub(crate) const fn is_allowed_string_char(c: u8) -> bool {
    matches!(c, 0x20..=0x7e)
}

macro_rules! impl_eq {
    ($ref:ty: $a:ty, $b:ty) => {
        impl PartialEq<$a> for $b {
            fn eq(&self, other: &$a) -> bool {
                <$ref as PartialEq>::eq(self, other)
            }
        }
        impl PartialEq<$b> for $a {
            fn eq(&self, other: &$b) -> bool {
                <$ref as PartialEq>::eq(self, other)
            }
        }
    };
}

macro_rules! impl_string_types {
    (
        owned: $Owned:ident,
        ref: $Ref:ident,
        const_fn: $const_fn:ident,
        desc: $desc:literal,
        $(is_allowed_start_char: $is_allowed_start_char:path,)?
        is_allowed_inner_char: $is_allowed_inner_char:path $(,)?
    ) => {
        impl $Ref {
            #[ref_cast::ref_cast_custom]
            const fn cast(v: &str) -> &Self;

            pub(crate) const fn validate_prefix(
                v: &[u8],
            ) -> Result<&Self, $crate::error::NonEmptyStringError> {
                let mut index = 0 $( + {
                    if v.is_empty() {
                        return Err($crate::error::NonEmptyStringError::empty());
                    }
                    if !$is_allowed_start_char(v[0]) {
                        return Err($crate::error::NonEmptyStringError::invalid_character(0));
                    }
                    1
                } )?;

                while index < v.len() && $is_allowed_inner_char(v[index]) {
                    index += 1;
                }

                let (prefix, _) = v.split_at(index);

                // TODO: The UTF-8 validation is redundant with the preceding character checks,
                // but its removal is only possible with unsafe code.
                let Ok(s) = std::str::from_utf8(prefix) else {
                    unreachable!()
                };
                Ok(Self::cast(s))
            }

            const fn validate(v: &[u8]) -> Result<&Self, $crate::error::NonEmptyStringError> {
                match Self::validate_prefix(v) {
                    Ok(prefix) => {
                        if prefix.0.len() == v.len() {
                            Ok(prefix)
                        } else {
                            Err($crate::error::NonEmptyStringError::invalid_character(
                                prefix.0.len(),
                            ))
                        }
                    }
                    Err(err) => Err(err),
                }
            }

            #[doc = concat!("Creates a `&", stringify!($Ref), "` from a `&str`.")]
            ///
            /// # Errors
            /// The error result reports the reason for any failed validation.
            #[allow(clippy::should_implement_trait)]
            pub fn from_str(v: &str) -> Result<&Self, $crate::Error> {
                Self::from_bytes(v.as_bytes())
            }

            #[doc = concat!("Creates a `&", stringify!($Ref), "` from a `&[u8]`.")]
            ///
            /// # Errors
            /// The error result reports the reason for any failed validation.
            pub fn from_bytes(v: &[u8]) -> Result<&Self, $crate::Error> {
                Ok(Self::validate(v)?)
            }

            // Like `from_str`, but assumes that the contents of the string have already
            // been validated.
            #[allow(dead_code, reason = "only used for StringRef")]
            pub(crate) fn from_validated_str(v: &str) -> &Self {
                debug_assert!(Self::validate(v.as_bytes()).is_ok());
                Self::cast(v)
            }

            #[doc = concat!("Creates a `&", stringify!($Ref), "`.")]
            ///
            /// This method is intended to be called from `const` contexts in which the
            #[doc = concat!("value is known to be valid. Use [`", stringify!($Ref), "::from_str`] for non-panicking")]
            /// conversions.
            ///
            /// # Panics
            ///
            /// Panics if the given value is invalid.
            #[must_use]
            pub const fn constant(v: &str) -> &Self {
                match Self::validate(v.as_bytes()) {
                    Ok(res) => res,
                    Err(err) => panic!("{}", err.msg()),
                }
            }

            #[doc = concat!("Returns the ", $desc, " as a `&str`.")]
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl ToOwned for $Ref {
            type Owned = $Owned;

            fn to_owned(&self) -> $Owned {
                $Owned(self.0.to_owned())
            }

            fn clone_into(&self, target: &mut $Owned) {
                self.0.clone_into(&mut target.0);
            }
        }

        impl std::borrow::Borrow<$Ref> for $Owned {
            fn borrow(&self) -> &$Ref {
                self
            }
        }

        impl std::ops::Deref for $Owned {
            type Target = $Ref;

            fn deref(&self) -> &$Ref {
                $Ref::cast(&self.0)
            }
        }

        impl From<$Owned> for std::string::String {
            fn from(v: $Owned) -> std::string::String {
                v.0
            }
        }

        impl TryFrom<std::string::String> for $Owned {
            type Error = $crate::Error;

            fn try_from(v: std::string::String) -> Result<$Owned, $crate::Error> {
                $Ref::validate(v.as_bytes())?;
                Ok($Owned(v))
            }
        }

        impl $Owned {
            #[doc = concat!("Creates a `", stringify!($Owned), "` from a `std::string::String`.")]
            ///
            /// Returns the original value if the conversion failed.
            ///
            /// # Errors
            /// The error result reports the reason for any failed validation.
            pub fn from_string(
                v: std::string::String,
            ) -> Result<Self, ($crate::Error, std::string::String)> {
                match $Ref::validate(v.as_bytes()) {
                    Ok(_) => Ok(Self(v)),
                    Err(err) => Err((err.into(), v)),
                }
            }

            // Like `from_string`, but assumes that the contents of the string have already
            // been validated.
            #[allow(dead_code, reason = "only used for String")]
            pub(crate) fn from_validated_string(v: std::string::String) -> Self {
                debug_assert!($Ref::validate(v.as_bytes()).is_ok());
                Self(v)
            }
        }

        #[doc = concat!("Creates a `&", stringify!($Ref), "`, panicking if the value is invalid.")]
        ///
        #[doc = concat!("This is a convenience free function for [`", stringify!($Ref), "::constant`].")]
        ///
        /// This method is intended to be called from `const` contexts in which the
        #[doc = concat!("value is known to be valid. Use [`", stringify!($Ref), "::from_str`] for non-panicking")]
        /// conversions.
        #[must_use]
        pub const fn $const_fn(v: &str) -> &$Ref {
            $Ref::constant(v)
        }

        impl std::fmt::Display for $Ref {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::fmt::Display for $Owned {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                <$Ref as std::fmt::Display>::fmt(self, f)
            }
        }

        impl_eq!($Ref: $Owned, $Ref);
        impl_eq!($Ref: $Owned, &$Ref);

        impl<'a> TryFrom<&'a str> for &'a $Ref {
            type Error = $crate::Error;

            fn try_from(v: &'a str) -> Result<&'a $Ref, $crate::Error> {
                $Ref::from_str(v)
            }
        }

        impl<'a> TryFrom<&'a [u8]> for &'a $Ref {
            type Error = $crate::Error;

            fn try_from(v: &'a [u8]) -> Result<&'a $Ref, $crate::Error> {
                $Ref::from_bytes(v)
            }
        }

        impl std::borrow::Borrow<str> for $Owned {
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        impl std::borrow::Borrow<str> for $Ref {
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        #[cfg(feature = "arbitrary")]
        impl<'a> arbitrary::Arbitrary<'a> for &'a $Ref {
            fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
                let bytes = <&[u8]>::arbitrary(u)?;
                <$Ref>::validate_prefix(bytes).map_err(|err| {
                    if $crate::Error::from(err).index().is_none() {
                        arbitrary::Error::NotEnoughData
                    } else {
                        arbitrary::Error::IncorrectFormat
                    }
                })
            }

            fn size_hint(_depth: usize) -> (usize, Option<usize>) {
                (0 $( + { let _ = $is_allowed_start_char; 1 } )?, None)
            }
        }

        #[cfg(feature = "arbitrary")]
        impl<'a> arbitrary::Arbitrary<'a> for $Owned {
            fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
                <&$Ref>::arbitrary(u).map(ToOwned::to_owned)
            }

            fn size_hint(depth: usize) -> (usize, Option<usize>) {
                <&$Ref as arbitrary::Arbitrary>::size_hint(depth)
            }
        }
    };
}
