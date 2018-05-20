extern crate byteorder;

use std::net::*;
use std::io::*;
use std::io::Write;
use core::str::FromStr;
use self::byteorder::{ByteOrder, BigEndian};
use encoding::bytes::Substream;

pub trait McnpConnectionTraits {
    fn send_fixed_chunk_u8(&mut self, val:u8) -> Result<()>;
    fn send_fixed_chunk_i16(&mut self, val:i16) -> Result<()>;
    fn send_fixed_chunk_i32(&mut self, val:i32) -> Result<()>;
    fn send_fixed_chunk_i64(&mut self, val:i64) -> Result<()>;
    fn send_fixed_chunk_f32(&mut self, val:f32) -> Result<()>;
    fn send_fixed_chunk_f64(&mut self, val:f64) -> Result<()>;
    fn send_fixed_chunk_u8_arr(&mut self, val:&[u8]) -> Result<()>;

    fn read_fixed_chunk_u8(&mut self) -> Result<u8>;
    fn read_fixed_chunk_i16(&mut self) -> Result<i16>;
    fn read_fixed_chunk_i32(&mut self) -> Result<i32>;
    fn read_fixed_chunk_i64(&mut self) -> Result<i64>;
    fn read_fixed_chunk_f32(&mut self) -> Result<f32>;
    fn read_fixed_chunk_f64(&mut self) -> Result<f64>;
    fn read_fixed_chunk_u8_arr(&mut self, bytes_to_read: usize) -> Result<Vec<u8>>;

    fn send_cause(&mut self, cause:i32) -> Result<()>;
    fn read_cause(&mut self) -> Result<i32>;

    fn start_variable_chunk(&mut self, chunk_length:i64) -> Result<()>;
    fn send_variable_chunk_part(&mut self, chunk_part:&[u8]) -> Result<()>;

    fn read_variable_chunk(&mut self) -> Result<Vec<u8>>;


    //optionals
    fn send_variable_chunk(&mut self, arr:&[u8]) -> Result<()>;
    fn read_variable_chunk_as_stream(&mut self) -> Result<(Substream<TcpStream>, i64)>;
    fn send_variable_chunk_from_stream(&mut self, stream: &mut Read, stream_length:i64) -> Result<()>;
}

#[derive(Debug)]
pub struct McnpConnection {
    socket: TcpStream,
}

impl McnpConnection {
    pub fn new_from_stream(stream: TcpStream) -> McnpConnection {
        McnpConnection {
            socket:stream,
        }
    }
    pub fn new(addr:&str, port:u16) -> McnpConnection {
        return McnpConnection::new_from_stream(TcpStream::connect(SocketAddr::new(IpAddr::from_str(addr).unwrap(), port)).unwrap());
    }
}

impl McnpConnectionTraits for McnpConnection {
    fn send_fixed_chunk_u8(&mut self, val: u8) -> Result<()> {
        return self.socket.write_all(&[val]);
    }

    fn send_fixed_chunk_i16(&mut self, val: i16) -> Result<()> {
        let mut buf = [0; 2];
        BigEndian::write_i16(&mut buf, val);
        return self.socket.write_all(&buf)
    }

    fn send_fixed_chunk_i32(&mut self, val: i32) -> Result<()> {
        let mut buf = [0; 4];
        BigEndian::write_i32(&mut buf, val);
        return self.socket.write_all(&buf)
    }

    fn send_fixed_chunk_i64(&mut self, val: i64) -> Result<()> {
        let mut buf = [0; 8];
        BigEndian::write_i64(&mut buf, val);
        return self.socket.write_all(&buf)
    }

    fn send_fixed_chunk_f32(&mut self, val: f32) -> Result<()> {
        let mut buf = [0; 4];
        BigEndian::write_f32(&mut buf, val);
        return self.socket.write_all(&buf)
    }

    fn send_fixed_chunk_f64(&mut self, val: f64) -> Result<()> {
        let mut buf = [0; 8];
        BigEndian::write_f64(&mut buf, val);
        return self.socket.write_all(&buf)
    }

    fn send_fixed_chunk_u8_arr(&mut self, val: &[u8]) -> Result<()> {
        return self.socket.write_all(val)
    }

    fn read_fixed_chunk_u8(&mut self) -> Result<u8> {
        let mut buf = [0_u8; 1];
        match self.socket.read(&mut buf) {
            Ok(1)   => return Ok(buf[0]),
            Err(e)  => return Err(e),
            _       => return Err(Error::new(ErrorKind::Other, "read wrong number of bytes"))
        }
    }

    fn read_fixed_chunk_i16(&mut self) -> Result<i16> {
        let mut buf = [0_u8; 2];
        match self.socket.read(&mut buf) {
            Ok(2)   => return Ok(BigEndian::read_i16(&buf)),
            Err(e)  => return Err(e),
            _       => return Err(Error::new(ErrorKind::Other, "read wrong number of bytes"))
        }
    }

    fn read_fixed_chunk_i32(&mut self) -> Result<i32> {
        let mut buf = [0_u8; 4];
        match self.socket.read(&mut buf) {
            Ok(4)   =>  return Ok(BigEndian::read_i32(&buf)),
            Err(e)  => return Err(e),
            _       => return Err(Error::new(ErrorKind::Other, "read wrong number of bytes"))
        }
    }

    fn read_fixed_chunk_i64(&mut self) -> Result<i64> {
        let mut buf = [0_u8; 8];
        match self.socket.read(&mut buf) {
            Ok(8)   => return Ok(BigEndian::read_i64(&buf)),
            Err(e)  => return Err(e),
            _       => return Err(Error::new(ErrorKind::Other, "read wrong number of bytes"))
        }
    }

    fn read_fixed_chunk_f32(&mut self) -> Result<f32> {
        let mut buf = [0_u8; 4];
        match self.socket.read(&mut buf) {
            Ok(4)   => return Ok(BigEndian::read_f32(&buf)),
            Err(e)  => return Err(e),
            _       => return Err(Error::new(ErrorKind::Other, "read wrong number of bytes"))
        }
    }

    fn read_fixed_chunk_f64(&mut self) -> Result<f64> {
        let mut buf = [0_u8; 8];
        match self.socket.read(&mut buf) {
            Ok(8)   => return Ok(BigEndian::read_f64(&buf)),
            Err(e)  => return Err(e),
            _       => return Err(Error::new(ErrorKind::Other, "read wrong number of bytes"))
        }
    }

    fn read_fixed_chunk_u8_arr(&mut self, bytes_to_read: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; bytes_to_read];
        let bytes_read = self.socket.read(&mut buf)?;
        if bytes_read != bytes_to_read {
            return Err(Error::new(ErrorKind::Other, "bytes_read != bytes_to_read"));
        }
        return Ok(buf)
    }

    fn send_cause(&mut self, cause: i32) -> Result<()> {
        self.send_fixed_chunk_i32(cause)
    }

    fn read_cause(&mut self) -> Result<i32> {
        self.read_fixed_chunk_i32()
    }

    fn start_variable_chunk(&mut self, chunk_length: i64) -> Result<()> {
        self.send_fixed_chunk_i64(chunk_length)
    }

    fn send_variable_chunk_part(&mut self, chunk_part: &[u8]) -> Result<()> {
        self.send_fixed_chunk_u8_arr(chunk_part)
    }

    fn read_variable_chunk(&mut self) -> Result<Vec<u8>> {
        let chunk_length = self.read_fixed_chunk_i64()?;
        if chunk_length < 0 { // array length was negative. This may indicate that the other side isn't able to fulfill the request
            Err(Error::new(ErrorKind::InvalidData, "reading a negative amount of bytes is difficult at best"))
        } else if chunk_length > 100_000_000 { //100 megabyte
            Err(Error::new(ErrorKind::InvalidData, "chunk length to big. Try using a stream to handle this much data."))
        } else {
            let mut buf = vec![0u8; chunk_length as usize];
            match self.socket.read_exact(&mut buf) {
                Ok(_) => return Ok(buf),
                Err(e) => return Err(e)
            }
        }
    }

    fn send_variable_chunk(&mut self, chunk: &[u8]) -> Result<()> {
        self.start_variable_chunk(chunk.len() as i64)?;
        self.send_variable_chunk_part(chunk)
    }

    fn read_variable_chunk_as_stream(&mut self) -> Result<(Substream<TcpStream>, i64)> {
        let chunk_length = self.read_fixed_chunk_i64()?;
        if chunk_length < 0 { // array length was negative. This may indicate that the other side isn't able to fulfill the request
            return Err(Error::new(ErrorKind::InvalidData, "reading a negative amount of bytes is difficult at best"));
        }

        let socket_clone = self.socket.try_clone().unwrap();
        return Ok((Substream::new_from_start(socket_clone, chunk_length as u64), chunk_length))
    }

    fn send_variable_chunk_from_stream(&mut self, stream: &mut Read, stream_length:i64) -> Result<()> {
        match self.start_variable_chunk(stream_length) {
            Ok(_) => {
                let buffer_size = 1024 * 4;
                let mut buffer = vec![0u8; buffer_size];

                let byte_counter = 0;
                while byte_counter < stream_length {
                    match stream.read(&mut buffer) {
                        Err(e) => return Err(e),
                        Ok(read) => {
                            if read > 0 {
                                self.send_variable_chunk_part(&buffer[0..read]).expect("error sending chunk part");
                            } else {
                                break;
                            }
                        }
                    }
                }
                Ok(())
            },
            Err(_) => {
                panic!("error sending start chunk - no proper error handling implemented");
            }
        }

//        match self.start_variable_chunk(chunk.len() as i64) {
//            Ok(_) => {
//                return self.send_variable_chunk_part(chunk);
//            },
//            Err(e) => {
//                return Err(e);
//            },
//        }
    }
}

//#[derive(Debug)]
//pub struct Substream<R:Read> {
//    orig_file:R,
//    cur_pos:u64,
//    end_pos:u64
//}
//impl<R:Read> Substream<R> {
//    pub fn new(mut orig:R, end:u64) -> Substream<R> {
//        Substream {
//            orig_file:orig,
//            cur_pos:0,
//            end_pos:end
//        }
//    }
//}
//impl<R:Read> Read for Substream<R> {
//    fn read(&mut self, buf: &mut [u8]) -> ::std::io::Result<usize> {
//        if self.cur_pos >= self.end_pos {
//            return Ok(0)
//        }
//        let remaining_len = self.end_pos-self.cur_pos;
//        match self.orig_file.read(buf) {
//            Ok(bytes_read) => {
//                self.cur_pos += bytes_read as u64;
//                return Ok(cmp::min(remaining_len as usize, bytes_read));
//            },
//            Err(e) => {
//                return Err(e);
//            },
//        }
//    }
//}