// Минимальный NBT-парсер (Java Edition, big endian) — хватает для servers.dat
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List(Vec<Value>),
    Compound(Vec<(String, Value)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Compound(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::ByteArray(b) => Some(b),
            _ => None,
        }
    }
}

pub fn parse(data: &[u8]) -> Result<Value, String> {
    // servers.dat обычно несжатый, но на всякий случай пробуем gzip
    let owned: Vec<u8> = if data.first() == Some(&0x0a) {
        data.to_vec()
    } else {
        decompress(data)?
    };
    let mut c = Cursor {
        data: &owned,
        pos: 0,
    };
    let ty = c.u8()?;
    if ty != 10 {
        return Err("корень NBT не compound".into());
    }
    let _name = c.string()?;
    c.compound_body()
}

fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(data)
        .read_to_end(&mut out)
        .map_err(|e| format!("gzip NBT: {e}"))?;
    Ok(out)
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if n > self.data.len() - self.pos {
            return Err("NBT обрезан".into());
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn be<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let s = self.take(N)?;
        let mut out = [0u8; N];
        out.copy_from_slice(s);
        Ok(out)
    }

    fn u16be(&mut self) -> Result<u16, String> {
        Ok(u16::from_be_bytes(self.be()?))
    }

    fn string(&mut self) -> Result<String, String> {
        let len = self.u16be()? as usize;
        let bytes = self.take(len)?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    fn sized(&self, count: usize, item_size: usize) -> Result<usize, String> {
        // Защита от inflate-атаки: длина не должна выходить за остаток данных
        let rest = self.data.len() - self.pos;
        if count as u64 > (rest / item_size.max(1) + 1) as u64 {
            return Err("NBT: подозрительно длинный массив".into());
        }
        Ok(count)
    }

    fn value(&mut self, ty: u8) -> Result<Value, String> {
        Ok(match ty {
            1 => Value::Byte(self.be::<1>()?[0] as i8),
            2 => Value::Short(i16::from_be_bytes(self.be()?)),
            3 => Value::Int(i32::from_be_bytes(self.be()?)),
            4 => Value::Long(i64::from_be_bytes(self.be()?)),
            5 => Value::Float(f32::from_be_bytes(self.be()?)),
            6 => Value::Double(f64::from_be_bytes(self.be()?)),
            7 => {
                let n = i32::from_be_bytes(self.be()?).max(0) as usize;
                let len = self.sized(n, 1)?;
                Value::ByteArray(self.take(len)?.to_vec())
            }
            8 => Value::String(self.string()?),
            9 => {
                let et = self.u8()?;
                let n = i32::from_be_bytes(self.be()?).max(0) as usize;
                let len = self.sized(n, 1)?;
                let mut items = Vec::with_capacity(len.min(1024));
                for _ in 0..len {
                    items.push(self.value(et)?);
                }
                Value::List(items)
            }
            10 => self.compound_body()?,
            11 => {
                let n = i32::from_be_bytes(self.be()?).max(0) as usize;
                let len = self.sized(n, 4)?;
                let mut items = Vec::with_capacity(len);
                for _ in 0..len {
                    items.push(i32::from_be_bytes(self.be()?));
                }
                Value::IntArray(items)
            }
            12 => {
                let n = i32::from_be_bytes(self.be()?).max(0) as usize;
                let len = self.sized(n, 8)?;
                let mut items = Vec::with_capacity(len);
                for _ in 0..len {
                    items.push(i64::from_be_bytes(self.be()?));
                }
                Value::LongArray(items)
            }
            _ => return Err(format!("неизвестный NBT-тег {ty}")),
        })
    }

    fn compound_body(&mut self) -> Result<Value, String> {
        let mut items = Vec::new();
        loop {
            let ty = self.u8()?;
            if ty == 0 {
                break;
            }
            let name = self.string()?;
            let v = self.value(ty)?;
            items.push((name, v));
        }
        Ok(Value::Compound(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Вспомогалки сборки NBT-байтов для тестов
    fn named_tag(ty: u8, name: &str, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![ty];
        out.extend_from_slice(&(name.len() as u16).to_be_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn str_payload(s: &str) -> Vec<u8> {
        let mut out = (s.len() as u16).to_be_bytes().to_vec();
        out.extend_from_slice(s.as_bytes());
        out
    }

    #[test]
    fn parses_servers_like_dat() {
        // ByteArray: длина, потом PNG-магия
        let mut icon = 4i32.to_be_bytes().to_vec();
        icon.extend_from_slice(&[137, 80, 78, 71]);

        let mut entry = named_tag(8, "name", &str_payload("Локальный сервер"));
        entry.extend_from_slice(&named_tag(8, "ip", &str_payload("127.0.0.1:25565")));
        entry.extend_from_slice(&named_tag(7, "icon", &icon));
        entry.push(0);

        // List<Compound>: тип элементов 10, длина 1, потом запись
        let mut list = (10u8).to_be_bytes().to_vec();
        list.extend_from_slice(&1i32.to_be_bytes());
        list.extend_from_slice(&entry);

        let mut data = vec![10u8];
        data.extend_from_slice(&(0u16).to_be_bytes());
        data.extend_from_slice(&named_tag(9, "servers", &list));
        data.push(0);

        let root = parse(&data).unwrap();
        let servers = root.get("servers").unwrap().as_list().unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(
            servers[0].get("name").unwrap().as_string(),
            Some("Локальный сервер")
        );
        assert_eq!(
            servers[0].get("ip").unwrap().as_string(),
            Some("127.0.0.1:25565")
        );
        assert_eq!(servers[0].get("icon").unwrap().as_bytes().unwrap().len(), 4);
    }

    #[test]
    fn truncated_data_is_error() {
        let data = [10u8, 0, 0, 1];
        assert!(parse(&data).is_err());
    }

    #[test]
    fn unknown_tag_is_error() {
        let mut data = vec![10u8, 0, 0];
        data.push(99);
        data.extend_from_slice(&(0u16).to_be_bytes());
        assert!(parse(&data).is_err());
    }

    #[test]
    fn huge_list_is_rejected() {
        let mut data = vec![10u8, 0, 0];
        data.extend_from_slice(&named_tag(9, "servers", &[9u8, 10]));
        data.extend_from_slice(&(2_000_000_000i32).to_be_bytes());
        assert!(parse(&data).is_err());
    }
}
