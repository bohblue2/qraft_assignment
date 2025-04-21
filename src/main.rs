// To run this code, create a new Cargo project and add the following to Cargo.toml:
//
// [dependencies]
// rust_decimal = "1.29"
//
// Then place this in src/lib.rs (or main.rs) and run `cargo test`.

use std::str;

pub fn decode_pack_bcd(encoded: &[u8], digit_count: usize) -> String {
    let mut digits = String::with_capacity(encoded.len() * 2);
    for &b in encoded {
        let high = (b >> 4) & 0x0F;
        let low  = b & 0x0F;
        // from_digit always returns Some for 0–9
        digits.push(char::from_digit(high as u32, 10).unwrap());
        digits.push(char::from_digit(low  as u32, 10).unwrap());
    }
    digits.chars().take(digit_count).collect()
}

#[derive(Debug, PartialEq)]
pub struct RealTimeQuote {
    pub price:  String,
    pub volume: String,
}

#[derive(Debug, PartialEq)]
pub struct Format6Record {
    pub esc_code:               u8,
    pub info_length:            u32,
    pub business_type:          String,
    pub format_code:            String,
    pub version:                String,
    pub transmission_sn:        String,
    pub stock_code:             String,
    pub matching_time:          String,
    pub disclosed_item_remarks: u8,
    pub rise_fall_remarks:      u8,
    pub status_remarks:         u8,
    pub accumulative_volume:    u32,
    pub real_time_quotes:       Vec<RealTimeQuote>,
    pub checksum:               u8,
    pub terminal_code:          [u8; 2],
}

pub fn parse_format6(raw: &[u8]) -> Format6Record {
    let mut idx = 0;
    // 1) ESC-CODE
    let esc_code = raw[idx];
    idx += 1;

    // 2) HEADER
    let info_length = {
        let s = decode_pack_bcd(&raw[idx..idx+2], 4);
        idx += 2;
        s.parse::<u32>().unwrap()
    };
    let business_type = {
        let s = decode_pack_bcd(&raw[idx..idx+1], 2);
        idx += 1;
        s
    };
    let format_code = {
        let s = decode_pack_bcd(&raw[idx..idx+1], 2);
        idx += 1;
        s
    };
    let version = {
        let s = decode_pack_bcd(&raw[idx..idx+1], 2);
        idx += 1;
        s
    };
    let transmission_sn = {
        let s = decode_pack_bcd(&raw[idx..idx+4], 8);
        idx += 4;
        s
    };

    // 3) BODY
    let stock_code = {
        let slice = &raw[idx..idx+6];
        idx += 6;
        let s = str::from_utf8(slice).unwrap();
        s.trim_end().to_string()
    };
    let matching_time = {
        let s = decode_pack_bcd(&raw[idx..idx+6], 12);
        idx += 6;
        s
    };
    let disclosed_item_remarks = raw[idx]; idx += 1;
    let rise_fall_remarks      = raw[idx]; idx += 1;
    let status_remarks         = raw[idx]; idx += 1;

    let accumulative_volume = {
        let s = decode_pack_bcd(&raw[idx..idx+4], 8);
        idx += 4;
        s.parse::<u32>().unwrap()
    };

    // 3.7 Real-time Quotes (here we parse exactly one entry)
    let price = {
        let s = decode_pack_bcd(&raw[idx..idx+5], 9);
        idx += 5;
        s
    };
    let volume = {
        let s = decode_pack_bcd(&raw[idx..idx+4], 8);
        idx += 4;
        s
    };
    let real_time_quotes = vec![ RealTimeQuote { price, volume } ];

    // 4) Checksum
    let checksum = raw[idx];
    idx += 1;

    // 5) Terminal Code
    let terminal_code = [ raw[idx], raw[idx+1] ];
    // idx += 2;

    Format6Record {
        esc_code,
        info_length,
        business_type,
        format_code,
        version,
        transmission_sn,
        stock_code,
        matching_time,
        disclosed_item_remarks,
        rise_fall_remarks,
        status_remarks,
        accumulative_volume,
        real_time_quotes,
        checksum,
        terminal_code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_basic_decoding() {
        let cases = vec![
            (vec![0x12, 0x34], 4, "1234"),
            (vec![0x00, 0x01], 4, "0001"),
            (vec![0x98, 0x76], 4, "9876"),
            (vec![0x12, 0x34, 0x56], 5, "12345"),
        ];
        for (encoded, digits, expected) in cases {
            let result = decode_pack_bcd(&encoded, digits);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_decimal_insertion() {
        let encoded = vec![0x12, 0x34, 0x56]; // "123456"
        let raw = decode_pack_bcd(&encoded, 6);
        assert_eq!(raw, "123456");

        // Insert decimal point after 3 digits
        let decimal_str = format!("{}.{}", &raw[..3], &raw[3..]);
        assert_eq!(decimal_str, "123.456");

        // Parse to Decimal for precise check
        let dec = Decimal::from_str(&decimal_str).unwrap();
        assert_eq!(dec.to_string(), "123.456");
    }

    #[test]
    fn test_parse_format6() {
        // Build raw_record exactly as in the spec example
        let mut raw = Vec::new();
        raw.push(0x1B); // ESC

        // HEADER
        raw.extend(&[0x00, 0x47]); // InfoLength = "0047"
        raw.push(0x01);            // Business Type "01"
        raw.push(0x06);            // Format Code "06"
        raw.push(0x04);            // Version "04"
        raw.extend(&[0x00,0x00,0x00,0x01]); // S/N "00000001"

        // BODY
        raw.extend(b"2330  ");             // StockCode
        raw.extend(&[0x09,0x30,0x15,0x12,0x34,0x56]); // Matching Time
        raw.push(0x89); // Disclosed Item Remarks
        raw.push(0x00); // Rise/Fall Remarks
        raw.push(0x80); // Status Remarks
        raw.extend(&[0x00,0x00,0x12,0x34]); // Accum Volume
        raw.extend(&[0x00,0x12,0x34,0x56,0x70]); // Price Field
        raw.extend(&[0x00,0x00,0x01,0x00]);       // Volume Field

        raw.push(0x5A); // Checksum
        raw.extend(&[0x0D,0x0A]); // Terminal Code

        let rec = parse_format6(&raw);

        assert_eq!(rec.esc_code, 0x1B);
        assert_eq!(rec.info_length, 47);
        assert_eq!(&rec.business_type, "01");
        assert_eq!(&rec.format_code,   "06");
        assert_eq!(&rec.version,       "04");
        assert_eq!(&rec.transmission_sn, "00000001");
        assert_eq!(&rec.stock_code,    "2330");
        assert_eq!(&rec.matching_time, "093015123456");
        assert_eq!(rec.disclosed_item_remarks, 0x89);
        assert_eq!(rec.rise_fall_remarks,      0x00);
        assert_eq!(rec.status_remarks,         0x80);
        assert_eq!(rec.accumulative_volume,    1234);
        assert_eq!(rec.real_time_quotes.len(), 1);
        assert_eq!(&rec.real_time_quotes[0].price,  "001234567");
        assert_eq!(&rec.real_time_quotes[0].volume, "00000100");
        assert_eq!(rec.checksum,  0x5A);
        assert_eq!(rec.terminal_code, [0x0D,0x0A]);
    }
}