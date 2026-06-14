// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use cu::pre::*;

pub struct BinReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BinReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        BinReader { data, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn read_u8(&mut self) -> cu::Result<u8> {
        if self.pos >= self.data.len() {
            cu::bail!("unexpected eof: expecting byte");
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    pub fn read_u32(&mut self) -> cu::Result<u32> {
        if self.pos + 4 > self.data.len() {
            cu::bail!("unexpected eof: expecting 4-byte unsigned 32-bit integer");
        }
        let bytes: [u8; 4] = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn read_f32(&mut self) -> cu::Result<f32> {
        if self.pos + 4 > self.data.len() {
            cu::bail!("unexpected eof: expecting 4-byte 32-bit float");
        }
        let bytes: [u8; 4] = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        Ok(f32::from_le_bytes(bytes))
    }
    pub fn read_slice(&mut self, length: usize) -> cu::Result<&'a [u8]> {
        if self.pos + length > self.data.len() {
            cu::bail!("unexpected eof: expecting {length} more bytes");
        }
        let bytes = &self.data[self.pos..self.pos + length];
        self.pos += length;
        Ok(bytes)
    }

    pub fn read_u32_len_utf8(&mut self) -> cu::Result<&'a str> {
        let len = cu::check!(self.read_u32(), "failed to read length of string")? as usize;
        if self.pos + len > self.data.len() {
            cu::bail!("unexpected eof: expecting a utf-8 encoded string of {len} bytes");
        }
        let bytes = &self.data[self.pos..self.pos + len];
        self.pos += len;
        let s = cu::check!(str::from_utf8(bytes), "failed to decode string as utf-8")?;
        Ok(s)
    }

    pub fn remaining(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }
}
