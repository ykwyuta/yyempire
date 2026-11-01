//! 記録（セーブデータ）の読み書き：外部クレートを使わない小さなバイナリ形式

pub struct Writer {
    pub buf: Vec<u8>,
}

impl Writer {
    pub fn new() -> Writer {
        Writer { buf: Vec::with_capacity(1 << 18) }
    }
    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    pub fn bool(&mut self, v: bool) {
        self.buf.push(v as u8);
    }
    /// 可変長の符号なし整数
    pub fn uv(&mut self, mut v: u64) {
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                self.buf.push(b);
                return;
            }
            self.buf.push(b | 0x80);
        }
    }
    pub fn u32(&mut self, v: u32) {
        self.uv(v as u64);
    }
    pub fn usize(&mut self, v: usize) {
        self.uv(v as u64);
    }
    pub fn i32(&mut self, v: i32) {
        self.uv(((v << 1) ^ (v >> 31)) as u32 as u64);
    }
    pub fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub fn f32(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub fn str(&mut self, s: &str) {
        self.usize(s.len());
        self.buf.extend_from_slice(s.as_bytes());
    }
    pub fn bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }
    /// 同じ値の並びを圧縮して書く
    pub fn rle(&mut self, data: &[u8]) {
        self.usize(data.len());
        let mut i = 0;
        while i < data.len() {
            let v = data[i];
            let mut n = 1;
            while i + n < data.len() && data[i + n] == v {
                n += 1;
            }
            self.u8(v);
            self.usize(n);
            i += n;
        }
    }
}

pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

pub type RResult<T> = Result<T, String>;

fn eof() -> String {
    t!("記録が途中で切れている", "The save is truncated", "存档不完整").to_string()
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Reader<'a> {
        Reader { data, pos: 0 }
    }
    pub fn u8(&mut self) -> RResult<u8> {
        let v = *self.data.get(self.pos).ok_or_else(eof)?;
        self.pos += 1;
        Ok(v)
    }
    pub fn bool(&mut self) -> RResult<bool> {
        Ok(self.u8()? != 0)
    }
    pub fn uv(&mut self) -> RResult<u64> {
        let mut v = 0u64;
        let mut sh = 0;
        loop {
            let b = self.u8()?;
            v |= ((b & 0x7f) as u64) << sh;
            if b & 0x80 == 0 {
                return Ok(v);
            }
            sh += 7;
            if sh > 63 {
                return Err(t!("記録が壊れている（整数）", "The save is damaged (integer)", "存档已损坏（整数）").into());
            }
        }
    }
    pub fn u32(&mut self) -> RResult<u32> {
        Ok(self.uv()? as u32)
    }
    pub fn usize(&mut self) -> RResult<usize> {
        Ok(self.uv()? as usize)
    }
    /// 配列の長さ（上限つき）
    pub fn len(&mut self, max: usize) -> RResult<usize> {
        let n = self.usize()?;
        if n > max {
            return Err(t!("記録が壊れている（長さ）", "The save is damaged (length)", "存档已损坏（长度）").into());
        }
        Ok(n)
    }
    pub fn i32(&mut self) -> RResult<i32> {
        let u = self.uv()? as u32;
        Ok(((u >> 1) as i32) ^ -((u & 1) as i32))
    }
    fn take(&mut self, n: usize) -> RResult<&'a [u8]> {
        if self.pos + n > self.data.len() {
            return Err(eof());
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    pub fn u64(&mut self) -> RResult<u64> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes(b.try_into().unwrap()))
    }
    pub fn f32(&mut self) -> RResult<f32> {
        let b = self.take(4)?;
        Ok(f32::from_le_bytes(b.try_into().unwrap()))
    }
    pub fn str(&mut self) -> RResult<String> {
        let n = self.len(1 << 20)?;
        let b = self.take(n)?;
        String::from_utf8(b.to_vec()).map_err(|_| t!("記録が壊れている（文字列）", "The save is damaged (text)", "存档已损坏（文字）").to_string())
    }
    pub fn bytes(&mut self, n: usize) -> RResult<&'a [u8]> {
        self.take(n)
    }
    pub fn rle(&mut self, expect: usize) -> RResult<Vec<u8>> {
        let n = self.usize()?;
        if n != expect {
            return Err(t!("記録の地図の大きさが合わない", "The map size in the save does not match", "存档的地图尺寸不符").into());
        }
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            let v = self.u8()?;
            let k = self.usize()?;
            if out.len() + k > n {
                return Err(t!("記録が壊れている（地図）", "The save is damaged (map)", "存档已损坏（地图）").into());
            }
            out.extend(std::iter::repeat(v).take(k));
        }
        Ok(out)
    }
}
