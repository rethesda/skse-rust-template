//! Character encoding shenanigans. Bethesda is very bad at utf-8, I am told.

use byte_slice_cast::AsSliceOf;
use cxx::CxxVector;
use textcode::{iso8859_15, iso8859_9};

/// This is a silly papyrus example. We choose to return -1 to signal
/// failure because our use case is as array indexes in papyrus.
pub fn string_to_int(number: String) -> i32 {
    if let Ok(parsed) = number.parse::<i32>() {
        parsed
    } else {
        -1
    }
}

// To test in game: install daegon
// player.additem xxxb15f4 1
// Sacrÿfev Tëliimi

/// Use this for null-terminated C strings.
pub fn cstr_to_utf8(bytes_ffi: &CxxVector<u8>) -> String {
    let bytes: Vec<u8> = bytes_ffi.iter().copied().collect();
    let bytes = if bytes.ends_with(&[0]) {
        let chopped = bytes.len() - 1;
        let mut tmp = bytes.clone();
        tmp.truncate(chopped);
        tmp
    } else {
        bytes
    };
    convert_to_utf8(bytes)
}

/// Get a valid Rust representation of this Windows string data by hook or by crook.
pub fn convert_to_utf8(bytes: Vec<u8>) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    let (encoding, _confidence, _language) = chardet::detect(&bytes);
    match encoding.as_str() {
        "utf-8" => String::from_utf8(bytes.clone())
            .unwrap_or_else(|_| String::from_utf8_lossy(&bytes).to_string()),
        "ISO-8859-9" => {
            let mut dst = String::new();
            iso8859_9::decode(bytes.as_slice(), &mut dst);
            dst
        }
        "ISO-8859-15" => {
            let mut dst = String::new();
            iso8859_15::decode(bytes.as_slice(), &mut dst);
            dst
        }
        _ => {
            let Ok(widebytes) = bytes.as_slice_of::<u16>() else {
                return String::from_utf8_lossy(bytes.as_slice()).to_string();
            };
            let mut utf8bytes: Vec<u8> = vec![0; widebytes.len()];
            let Ok(_c) = ucs2::decode(widebytes, &mut utf8bytes) else {
                return String::from_utf8_lossy(bytes.as_slice()).to_string();
            };
            String::from_utf8(utf8bytes.clone())
                .unwrap_or_else(|_| String::from_utf8_lossy(utf8bytes.as_slice()).to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_data_is_untouched() {
        let example = "Sacrÿfev Tëliimi";
        let converted = convert_to_utf8(example.as_bytes().to_vec());
        assert_eq!(converted, example);
        let ex2 = "おはよう";
        let convert2 = convert_to_utf8(ex2.as_bytes().to_vec());
        assert_eq!(convert2, ex2);
        let ex3 = "Zażółć gęślą jaźń";
        let convert3 = convert_to_utf8(ex3.as_bytes().to_vec());
        assert_eq!(convert3, ex3);
    }

    #[test]
    fn iso8859_is_decoded() {
        // This is the example above (from the Daegon mod), in its expression
        // as windows codepage bytes. This test is the equivalent of me testing
        // that the textcode mod works, but I am feeling timid.
        let bytes: Vec<u8> = vec![
            0x53, 0x61, 0x63, 0x72, 0xff, 0x66, 0x65, 0x76, 0x20, 0x54, 0xeb, 0x6c, 0x69, 0x69,
            0x6d, 0x69,
        ];
        assert!(String::from_utf8(bytes.clone()).is_err());
        let utf8_version = "Sacrÿfev Tëliimi".to_string();
        let converted = convert_to_utf8(bytes.clone());
        assert_eq!(converted, utf8_version);
    }

    #[test]
    fn ucs2_is_decoded() {
        // UCS2 data might come from translation files. (Or I fear it might, anyway.
        // I have not proven this yet.) This is a fixed-width encoding that pads 8-bit
        // characters with 0. `chardet` guesses that this string is ascii, which is wrong.
        let bytes = vec![
            36, 0, 83, 0, 111, 0, 117, 0, 108, 0, 115, 0, 121, 0, 72, 0, 85, 0, 68, 0, 9, 0, 83, 0,
            111, 0, 117, 0, 108, 0, 115, 0, 121, 0, 32, 0, 72, 0, 85, 0, 68, 0,
        ];
        assert_eq!(bytes.len(), 42);
        let converted = convert_to_utf8(bytes.clone());
        assert_eq!(converted.len(), bytes.len() / 2);
        assert_eq!(converted, "$SoulsyHUD	Soulsy HUD");
    }
}
