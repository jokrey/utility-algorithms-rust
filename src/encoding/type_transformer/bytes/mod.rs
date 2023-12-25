extern crate byteorder;

use std;

use crate::encoding::tag_based::bytes::libae::LIbae;
use crate::encoding::tag_based::bytes::libae::LIbaeTraits;
use crate::encoding::type_transformer::bytes::booleans::detransform_booleans;
use crate::encoding::type_transformer::bytes::booleans::transform_booleans;

use self::byteorder::{BigEndian, ByteOrder};

///:author jokrey

#[cfg(test)]
pub mod booleans;

trait Transform<SF> {
    fn transform(&self) -> SF;
}
trait DeTransform<SF> {
    fn detransform(raw:&SF) -> Self;
}
trait DeTransformBytes
    where Self: std::marker::Sized {
    fn detransform(raw:&Vec<u8>) -> Self {
        DeTransformBytes::detransform_from(&raw[..])
    }
    fn detransform_from(raw:&[u8]) -> Self;
}
trait FromToTransform<SF> : Transform<SF> + DeTransform<SF> {}

impl Transform<Vec<u8>> for bool {
    fn transform(&self) -> Vec<u8> {
        if *self {
            vec![1]
        } else {
            vec![0]
        }
    }
}
impl DeTransformBytes for bool {
    fn detransform_from(raw: &[u8]) -> Self {
        raw[0] == 1
    }
}
//impl Transform<Vec<u8>> for [bool] {
//    fn transform(&self) -> Vec<u8> {
//        transform_booleans(self)
//    }
//}
impl Transform<Vec<u8>> for Vec<bool> {
    fn transform(&self) -> Vec<u8> {
        transform_booleans(self)
    }
}
impl DeTransformBytes for Vec<bool> {
    fn detransform_from(raw: &[u8]) -> Self {
        detransform_booleans(raw)
    }
}

impl Transform<Vec<u8>> for i16 {
    fn transform(&self) -> Vec<u8> {
        let mut b = vec![0u8;2];
        BigEndian::write_i16(&mut b, *self);
        b
    }
}
impl DeTransformBytes for i16 {
    fn detransform_from(raw: &[u8]) -> Self {
        BigEndian::read_i16(raw)
    }
}
//impl Transform<Vec<u8>> for [i16] {
//    fn transform(&self) -> Vec<u8> {
//        transform_array(self, 2)
//    }
//}
impl Transform<Vec<u8>> for Vec<i16> {
    fn transform(&self) -> Vec<u8> {
        transform_array(self, 2)
    }
}
impl DeTransformBytes for Vec<i16> {
    fn detransform_from(raw: &[u8]) -> Self {
        detransform_array(raw, 2)
    }
}

impl Transform<Vec<u8>> for i32 {
    fn transform(&self) -> Vec<u8> {
        let mut b = vec![0u8;4];
        BigEndian::write_i32(&mut b, *self);
        b
    }
}
impl DeTransformBytes for i32 {
    fn detransform_from(raw: &[u8]) -> Self {
        BigEndian::read_i32(raw)
    }
}
//impl Transform<Vec<u8>> for [i32] {
//    fn transform(&self) -> Vec<u8> {
//        transform_array(self, 4)
//    }
//}
impl Transform<Vec<u8>> for Vec<i32> {
    fn transform(&self) -> Vec<u8> {
        transform_array(self, 4)
    }
}
impl DeTransformBytes for Vec<i32> {
    fn detransform_from(raw: &[u8]) -> Self {
        detransform_array(raw, 4)
    }
}

impl Transform<Vec<u8>> for i64 {
    fn transform(&self) -> Vec<u8> {
        let mut b = vec![0u8;8];
        BigEndian::write_i64(&mut b, *self);
        b
    }
}
impl DeTransformBytes for i64 {
    fn detransform_from(raw: &[u8]) -> Self {
        BigEndian::read_i64(raw)
    }
}
//impl Transform<Vec<u8>> for [i64] {
//    fn transform(&self) -> Vec<u8> {
//        transform_array(self, 8)
//    }
//}
impl Transform<Vec<u8>> for Vec<i64> {
    fn transform(&self) -> Vec<u8> {
        transform_array(self, 8)
    }
}
impl DeTransformBytes for Vec<i64> {
    fn detransform_from(raw: &[u8]) -> Self {
        detransform_array(raw, 8)
    }
}

impl Transform<Vec<u8>> for f32 {
    fn transform(&self) -> Vec<u8> {
        let mut b = vec![0u8;4];
        BigEndian::write_f32(&mut b, *self);
        b
    }
}
impl DeTransformBytes for f32 {
    fn detransform_from(raw: &[u8]) -> Self {
        BigEndian::read_f32(raw)
    }
}
//impl Transform<Vec<u8>> for [f32] {
//    fn transform(&self) -> Vec<u8> {
//        transform_array(self, 4)
//    }
//}
impl Transform<Vec<u8>> for Vec<f32> {
    fn transform(&self) -> Vec<u8> {
        transform_array(self, 4)
    }
}
impl DeTransformBytes for Vec<f32> {
    fn detransform_from(raw: &[u8]) -> Self {
        detransform_array(raw, 4)
    }
}

impl Transform<Vec<u8>> for f64 {
    fn transform(&self) -> Vec<u8> {
        let mut b = vec![0u8;8];
        BigEndian::write_f64(&mut b, *self);
        b
    }
}
impl DeTransformBytes for f64 {
    fn detransform_from(raw: &[u8]) -> Self {
        BigEndian::read_f64(raw)
    }
}
//impl Transform<Vec<u8>> for [f64] {
//    fn transform(&self) -> Vec<u8> {
//        transform_array(self, 8)
//    }
//}
impl Transform<Vec<u8>> for Vec<f64> {
    fn transform(&self) -> Vec<u8> {
        transform_array(self, 8)
    }
}
impl DeTransformBytes for Vec<f64> {
    fn detransform_from(raw: &[u8]) -> Self {
        detransform_array(raw, 8)
    }
}

impl Transform<Vec<u8>> for String {
    fn transform(&self) -> Vec<u8> {
        Vec::from(self.as_bytes())
    }
}
impl DeTransformBytes for String {
    fn detransform_from(raw: &[u8]) -> Self {
        String::from_utf8(raw.to_vec()).unwrap()
    }
}
impl Transform<Vec<u8>> for str {
    fn transform(&self) -> Vec<u8> {
        Vec::from(self.as_bytes())
    }
}
impl Transform<Vec<u8>> for &'static str {
    fn transform(&self) -> Vec<u8> {
        Vec::from(self.as_bytes())
    }
}

fn transform_array<T:Transform<Vec<u8>>+Sized+Clone>(ts:&[T], element_size:usize) -> Vec<u8> {
    let mut bs = Vec::with_capacity(element_size * ts.len());
    for t in ts {
        for c in t.clone().transform() {
            bs.push(c);
        }
    }
    return bs
}
fn detransform_array<T:DeTransformBytes+Sized+Clone>(arr:&[u8], element_size:usize) -> Vec<T> {
    let size = arr.len() / element_size;
    let mut result = Vec::with_capacity(size);
    for i in 0..size {
        let e_arr = &arr[i*element_size..((i+1)*element_size)];
        result.push(T::detransform_from(e_arr));
    }
    return result;
}

fn transform_any_array<T:Transform<Vec<u8>>>(ts:&[T]) -> Vec<u8> {
    let mut libae = LIbae::ram();
    for t in ts {
        libae.li_encode_single(&t.transform()).unwrap();
    }
    libae.get_content().unwrap()
}
fn detransform_any_array<T:DeTransformBytes>(raw: &[u8]) -> Vec<T> {
    let mut result = Vec::with_capacity(25);
    let mut libae = LIbae::ram();
    libae.set_content(raw).unwrap();
    for raw_part in libae {
        result.push(T::detransform_from(&raw_part));
    }
    result
}







#[test]
fn test() {
    let ob = true;
    let eb = ob.transform();
    let db = bool::detransform(&eb);
    assert_eq!(ob, db);


    let o1 = vec![true, false, false, true, false, true, true, true, true];
    assert_eq!(o1, Vec::<bool>::detransform(&o1.transform()));
    let o3 = vec![1i16,2,3,4,5,6,7];
    assert_eq!(o3, Vec::<i16>::detransform(&o3.transform()));
    let o4 = vec![1,2,3,4,5,6,7];
    assert_eq!(o4, Vec::<i32>::detransform(&o4.transform()));
    let o5 = vec![1i64,2,3,4,5,6,7];
    assert_eq!(o5, Vec::<i64>::detransform(&o5.transform()));
    let o6 = vec![1f32,2.0,3.0,4.0,5.0,6.0,7.0];
    assert_eq!(o6, Vec::<f32>::detransform(&o6.transform()));
    let o7 = vec![1f64,2.0,3.0,4.0,5.0,6.0,7.0];
    assert_eq!(o7, Vec::<f64>::detransform(&o7.transform()));
//    char[] o8 = new char[] {'a', 'x', '?', 'ä', 'í', '1'};
//    assert_eq!(o8, detransform(transform(o8), o8.getClass()));

    let p1 = true;
    assert_eq!(p1, bool::detransform(&p1.transform()));
    let p3:i16 = 13000;
    assert_eq!(p3, i16::detransform(&p3.transform()));
    let p4 = 356234513;
    assert_eq!(p4, i32::detransform(&p4.transform()));
    let p5:i64 = 45382344534513;
    assert_eq!(p5, i64::detransform(&p5.transform()));
    let p6:f32 = 133242534675657.123123123;
    assert_eq!(p6, f32::detransform(&p6.transform()));
    let p7 = 9865756756756756756753713.213123523234;
    assert_eq!(p7, f64::detransform(&p7.transform()));
//    char p8 = 'ó';
//    assert_eq!(new Character(p8), detransform(transform(p8), char.class));
//    assert_eq!(new Character(p8), detransform(transform(p8), Character.class));
    let p9 = "asfd lakzrn34vz3vzg874zvgae4b 7bzg8osez g74zgeagh847hse i hgseuhv784hv";
    assert_eq!(p9, String::detransform(&p9.transform()));

    //recursively supported arrays
    let a1 = vec![p9, "213123", "ä+sdäf+sdäf#+däsf+äsdvf", "test", ""];
    assert_eq!(a1, detransform_any_array(&transform_any_array(&a1)) as Vec<String>); }