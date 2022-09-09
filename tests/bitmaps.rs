use bitvec::prelude::*;

#[test]
pub fn test_bit_chunks_64_8() {
    // we want to test converting a 8x8 byte array to a 8x8 byte array
    let bitmap = (0..64).into_iter().map(|x| x as u8).collect::<Vec<u8>>();

    let mut i = 0;

    for chunk in bitmap.view_bits::<Msb0>().chunks(64).into_iter() {
        let mut b = [0u8; 8];
        for (idx, bit) in chunk.into_iter().enumerate() {
            let byte = idx / 8;
            let shift = 7 - idx % 8;
            if *bit {
                b[byte] |= 1 << shift;
            }
        }
        for (idx, byte) in b.iter().enumerate() {
            assert_eq!(*byte, i);
            i += 1;
        }
    }
}

#[test]
pub fn test_bit_chunks_4_15() {
    // we want to test converting a 8x8 byte array to a 8x8 byte array
    let bitmap = (0..4).into_iter().map(|x| x as u8).collect::<Vec<u8>>();
    let vals: [u8; 5] = [0, 0, 129, 0, 192]; // 192 = 1 1 MSB

    let mut i = 0;

    for chunk in bitmap.view_bits::<Msb0>().chunks(15).into_iter() {
        let mut b = [0u8; 2];
        for (idx, bit) in chunk.into_iter().enumerate() {
            let byte = idx / 8;
            let shift = 7 - idx % 8;
            if *bit {
                b[byte] |= 1 << shift;
            }
        }
        println!("chunk: {:?}, b: {:?}", chunk, b);
        for (idx, byte) in b.iter().enumerate() {
            assert_eq!(*byte, vals[i]);
            i += 1;
        }
    }
}
