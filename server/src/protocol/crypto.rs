/// RC4 stream cipher — must match Badang's `payload/encrypt.rs:rc4_crypt` exactly.
pub fn rc4_crypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut s: [u8; 256] = core::array::from_fn(|i| i as u8);
    let mut j: u8 = 0;
    for i in 0..256 {
        j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
        s.swap(i, j as usize);
    }
    let mut i: u8 = 0;
    j = 0;
    data.iter()
        .map(|&b| {
            i = i.wrapping_add(1);
            j = j.wrapping_add(s[i as usize]);
            s.swap(i as usize, j as usize);
            let k = s[s[i as usize].wrapping_add(s[j as usize]) as usize];
            b ^ k
        })
        .collect()
}

pub fn encrypt(plaintext: &[u8], key: &[u8]) -> Vec<u8> {
    rc4_crypt(plaintext, key)
}

pub fn decrypt(ciphertext: &[u8], key: &[u8]) -> Vec<u8> {
    rc4_crypt(ciphertext, key)
}
