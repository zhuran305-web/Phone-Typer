use qrcode::QrCode;
use qrcode::types::Color;

pub struct QrMatrix {
    pub size: usize,
    pub dark: Vec<bool>,
}

pub fn make_matrix(content: &str) -> Option<QrMatrix> {
    match QrCode::new(content.as_bytes()) {
        Ok(qr) => {
            let size = qr.width();
            let mut dark = Vec::with_capacity(size * size);
            for y in 0..size {
                for x in 0..size {
                    dark.push(qr[(x, y)] == Color::Dark);
                }
            }
            Some(QrMatrix { size, dark })
        }
        Err(_) => None,
    }
}

pub fn print_ascii(content: &str) {
    match QrCode::new(content.as_bytes()) {
        Ok(qr) => {
            for y in 0..qr.width() {
                let mut line = String::new();
                for x in 0..qr.width() {
                    let dark = qr[(x, y)] == Color::Dark;
                    line.push(if dark { ' ' } else { '█' });
                }
                println!("{}", line);
            }
        }
        Err(_) => println!("二维码生成失败"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_size_correct() {
        let m = make_matrix("http://127.0.0.1:8766").unwrap();
        assert!(m.size >= 21, "最小 QR 版本 1 为 21×21（含 quiet zone）");
        assert_eq!(m.dark.len(), m.size * m.size);
    }

    #[test]
    fn test_ascii_output_nonempty() {
        let m = make_matrix("test").unwrap();
        assert!(!m.dark.is_empty());
    }

    #[test]
    fn test_empty_string_still_produces_matrix() {
        let m = make_matrix("");
        assert!(m.is_some(), "空字符串仍可生成二维码");
    }

    #[test]
    fn test_matrix_has_both_colors() {
        let m = make_matrix("https://example.com/longer/path?query=1").unwrap();
        let has_dark = m.dark.iter().any(|&b| b);
        let has_light = m.dark.iter().any(|&b| !b);
        assert!(has_dark && has_light, "二维码应同时包含黑白模块");
    }
}
