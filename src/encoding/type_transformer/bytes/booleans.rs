
pub fn transform_booleans(val:&[bool]) -> Vec<u8> {
    if val.len() == 0 {
        vec![]
    } else {
        let number_of_bits_to_store = val.len() + 3;
        let number_of_over_bits:isize = val.len() as isize % 8;

        let mut bs:Vec<u8> = Vec::with_capacity(number_of_bits_to_store/8 + (if number_of_bits_to_store % 8 == 0 {0} else {1}));
        bs.push(((number_of_over_bits-1) << (8 - 3)) as u8);

        for i in 0..val.len() {
            let bit_index = i+3;
            let byte_index = bit_index/8;
            let bit_index_in_byte = bit_index % 8;
            if bit_index_in_byte == 0 {
                bs.push(0);
            }
            if val[i] {
                bs[byte_index] = set_bit(bs[byte_index], 7 - bit_index_in_byte);
            }
        }
        return bs
    }
}

pub fn detransform_booleans(en:&[u8]) -> Vec<bool> {
    if en.len() == 0 {
        vec![]
    } else {
        let number_of_over_bits = ((en[0] >> 5 & 0b111 ) + 1) % 8;

        let size = (en.len() - 1 ) * 8 + number_of_over_bits as usize;
        let mut results = Vec::with_capacity(size);

        for i in 0..size {
            let bit_index = i+3; //jump bytes occupied by over bit indicator
            let byte_index = bit_index/8;
            let bit_index_in_byte = bit_index % 8;
            results.push(get_bit(en[byte_index], 7 - bit_index_in_byte) == 1);
        }

        return results
    }
}


fn set_bit(n:u8, k:usize) -> u8 {
    n | (1 << k)
}
fn get_bit(n:u8, k:usize) -> u8 {
    (n >> k) & 1
}


#[test]
fn test() {
    let os = vec![
        vec![],vec![true],vec![false],vec![true, false],vec![true, true, true],vec![false, true, false, true],vec![false, false, false],
        vec![false, false, false, false, false, false, false, false],vec![true, true, true, true, true, true, true, true],
        vec![false, false, true, false, true, false, true, false],vec![true, true, false, false, false, false, true, true],
        vec![true, false, true, false, true, false, true, false, false, true],
        vec![false, true, false, false, false, false, false, false, true, true, true, true, true, true, true, true],
        vec![true, false, true, false, true, false, true, false, true, true, false, false, false, false, true, true,
            false, false, true, false, true, true, true, false, true, true, false, false, false, true, true, true,
            false, true, true, false, true, false, true, false, true, true, false, false, false, false, true, true,
            true, false, true, false, true, false, true, false, true, true, false, true, false, false, true, true,
            false, false, true, false, true, false, true, false, true, true, false, false, false, false, true, true,
            false, true, true, false, true, true, true, false, true, true, false, false, false, false, true, true,
            false, false, true, false, true, false, true, false, true, true, false, false, false, false, true, true,
            true, false, true, false, true, false, true, false, true, true, false, true, false, false, true, true,
            false, false, true, false, true, false, true, false, true, true, false, true, false, false, true, true,
            false, false, true, false, true, false, true, false, true, true, false, false, false, true, true, true,
            false, false, true, true, true, false, true, false, true, true, false, false, false, false, true, true,
            true, false, true, false, true, false, true, false, true, true, false, false, true, false, true, true,
            false, false, true, false, true, false, true, false, true, true, true, false, false, false, true, true,
            false, false, true, false, true, false, true, false, true, true, false, false, false, false, true, true],
    ];

    for o in os {
        let t = transform_booleans(&o);
        let d = detransform_booleans(&t);

        assert_eq!(o, d);
    }
}