use crate::{consts::*, params::*, symmetric::*};
use core::arch::x86_64::*;

pub(crate) const REJ_UNIFORM_AVX_NBLOCKS: usize =
  (12 * KYBER_N / 8 * (1 << 12) / KYBER_Q + XOF_BLOCKBYTES) / XOF_BLOCKBYTES;
const REJ_UNIFORM_AVX_BUFLEN: usize = REJ_UNIFORM_AVX_NBLOCKS * XOF_BLOCKBYTES;

pub unsafe fn _mm256_cmpge_epu16(a: __m256i, b: __m256i) -> __m256i
{
  _mm256_cmpeq_epi16(_mm256_max_epu16(a, b), a)
}

pub unsafe fn _mm_cmpge_epu16(a: __m128i, b: __m128i) -> __m128i
{
  _mm_cmpeq_epi16(_mm_max_epu16(a, b), a)
}

pub unsafe fn rej_uniform_avx(r: &mut [i16], buf: &[u8]) -> usize
{
  let mut ctr = 0;
  let mut pos = 0;
  let mut good: usize;
  let (mut val0, mut val1);
  let (mut f0, mut f1, mut g0, mut g1, mut g2, mut g3);
  let (mut f, mut t, mut pilo, mut pihi);
  let qdata_ptr = QDATA.coeffs[_16XQ..].as_ptr();
  let bound = _mm256_load_si256(qdata_ptr as *const __m256i);
  let ones = _mm256_set1_epi8(1);
  let mask = _mm256_set1_epi16(0xFFF);
  let idx8 = _mm256_set_epi8(
    15, 14, 14, 13, 12, 11, 11, 10, 9, 8, 8, 7, 6, 5, 5, 4, 11, 10, 10, 9, 8,
    7, 7, 6, 5, 4, 4, 3, 2, 1, 1, 0,
  );
  while ctr <= KYBER_N - 32 && pos <= REJ_UNIFORM_AVX_BUFLEN - 48 {
    f0 = _mm256_loadu_si256(buf[pos..].as_ptr() as *const __m256i);
    f1 = _mm256_loadu_si256(buf[pos + 24..].as_ptr() as *const __m256i);
    f0 = _mm256_permute4x64_epi64(f0, 0x94);
    f1 = _mm256_permute4x64_epi64(f1, 0x94);
    f0 = _mm256_shuffle_epi8(f0, idx8);
    f1 = _mm256_shuffle_epi8(f1, idx8);
    g0 = _mm256_srli_epi16(f0, 4);
    g1 = _mm256_srli_epi16(f1, 4);
    f0 = _mm256_blend_epi16(f0, g0, 0xAA);
    f1 = _mm256_blend_epi16(f1, g1, 0xAA);
    f0 = _mm256_and_si256(f0, mask);
    f1 = _mm256_and_si256(f1, mask);
    pos += 48;

    g0 = _mm256_cmpgt_epi16(bound, f0);
    g1 = _mm256_cmpgt_epi16(bound, f1);

    g0 = _mm256_packs_epi16(g0, g1);
    good = _mm256_movemask_epi8(g0) as usize;

    let mut l0 =
      _mm_loadl_epi64(IDX[(good >> 0) & 0xFF].as_ptr() as *const __m128i);
    g0 = _mm256_castsi128_si256(l0);
    let mut l1 =
      _mm_loadl_epi64(IDX[(good >> 8) & 0xFF].as_ptr() as *const __m128i);
    g1 = _mm256_castsi128_si256(l1);

    l0 = _mm_loadl_epi64(IDX[(good >> 16) & 0xFF].as_ptr() as *const __m128i);
    g0 = _mm256_inserti128_si256(g0, l0, 1);
    l1 = _mm_loadl_epi64(IDX[(good >> 24) & 0xFF].as_ptr() as *const __m128i);
    g1 = _mm256_inserti128_si256(g1, l1, 1);

    g2 = _mm256_add_epi8(g0, ones);
    g3 = _mm256_add_epi8(g1, ones);
    g0 = _mm256_unpacklo_epi8(g0, g2);
    g1 = _mm256_unpacklo_epi8(g1, g3);

    f0 = _mm256_shuffle_epi8(f0, g0);
    f1 = _mm256_shuffle_epi8(f1, g1);

    _mm_storeu_si128(
      r[ctr..].as_mut_ptr() as *mut __m128i,
      _mm256_castsi256_si128(f0),
    );
    ctr += _popcnt32(((good >> 0) & 0xFF) as i32) as usize;
    _mm_storeu_si128(
      r[ctr..].as_mut_ptr() as *mut __m128i,
      _mm256_extracti128_si256(f0, 1),
    );
    ctr += _popcnt32(((good >> 16) & 0xFF) as i32) as usize;
    _mm_storeu_si128(
      r[ctr..].as_mut_ptr() as *mut __m128i,
      _mm256_castsi256_si128(f1),
    );
    ctr += _popcnt32(((good >> 8) & 0xFF) as i32) as usize;
    _mm_storeu_si128(
      r[ctr..].as_mut_ptr() as *mut __m128i,
      _mm256_extracti128_si256(f1, 1),
    );
    ctr += _popcnt32(((good >> 24) & 0xFF) as i32) as usize;
  }

  while ctr <= KYBER_N - 8 && pos <= REJ_UNIFORM_AVX_BUFLEN - 12 {
    f = _mm_loadu_si128(buf[pos..].as_ptr() as *const __m128i);
    f = _mm_shuffle_epi8(f, _mm256_castsi256_si128(idx8));
    t = _mm_srli_epi16(f, 4);
    f = _mm_blend_epi16(f, t, 0xAA);
    f = _mm_and_si128(f, _mm256_castsi256_si128(mask));
    pos += 12;

    t = _mm_cmpgt_epi16(_mm256_castsi256_si128(bound), f);
    good = _mm_movemask_epi8(t) as usize;

    let good = _pext_u32(good as u32, 0x5555) as usize;
    pilo = _mm_loadl_epi64(IDX[good][..].as_ptr() as *const __m128i);
    pihi = _mm_add_epi8(pilo, _mm256_castsi256_si128(ones));
    pilo = _mm_unpacklo_epi8(pilo, pihi);
    f = _mm_shuffle_epi8(f, pilo);
    _mm_storeu_si128(r[ctr..].as_mut_ptr() as *mut __m128i, f);
    ctr += _popcnt32(good as i32) as usize;
  }

  while ctr < KYBER_N && pos <= REJ_UNIFORM_AVX_BUFLEN - 3 {
    val0 = (buf[pos + 0] >> 0) as u16 | ((buf[pos + 1] as u16) << 8) & 0xFFF;
    val1 = (buf[pos + 1] >> 4) as u16 | ((buf[pos + 2] as u16) << 4);
    pos += 3;

    if (val0 as usize) < KYBER_Q {
      r[ctr] = val0 as i16;
      ctr += 1;
    }
    if (val1 as usize) < KYBER_Q && ctr < KYBER_N {
      r[ctr] = val1 as i16;
      ctr += 1;
    }
  }
  ctr
}

#[rustfmt::skip]
const IDX: [[i8; 8]; 256] = [
  [-1, -1, -1, -1, -1, -1, -1, -1],
  [ 0, -1, -1, -1, -1, -1, -1, -1],
  [ 2, -1, -1, -1, -1, -1, -1, -1],
  [ 0,  2, -1, -1, -1, -1, -1, -1],
  [ 4, -1, -1, -1, -1, -1, -1, -1],
  [ 0,  4, -1, -1, -1, -1, -1, -1],
  [ 2,  4, -1, -1, -1, -1, -1, -1],
  [ 0,  2,  4, -1, -1, -1, -1, -1],
  [ 6, -1, -1, -1, -1, -1, -1, -1],
  [ 0,  6, -1, -1, -1, -1, -1, -1],
  [ 2,  6, -1, -1, -1, -1, -1, -1],
  [ 0,  2,  6, -1, -1, -1, -1, -1],
  [ 4,  6, -1, -1, -1, -1, -1, -1],
  [ 0,  4,  6, -1, -1, -1, -1, -1],
  [ 2,  4,  6, -1, -1, -1, -1, -1],
  [ 0,  2,  4,  6, -1, -1, -1, -1],
  [ 8, -1, -1, -1, -1, -1, -1, -1],
  [ 0,  8, -1, -1, -1, -1, -1, -1],
  [ 2,  8, -1, -1, -1, -1, -1, -1],
  [ 0,  2,  8, -1, -1, -1, -1, -1],
  [ 4,  8, -1, -1, -1, -1, -1, -1],
  [ 0,  4,  8, -1, -1, -1, -1, -1],
  [ 2,  4,  8, -1, -1, -1, -1, -1],
  [ 0,  2,  4,  8, -1, -1, -1, -1],
  [ 6,  8, -1, -1, -1, -1, -1, -1],
  [ 0,  6,  8, -1, -1, -1, -1, -1],
  [ 2,  6,  8, -1, -1, -1, -1, -1],
  [ 0,  2,  6,  8, -1, -1, -1, -1],
  [ 4,  6,  8, -1, -1, -1, -1, -1],
  [ 0,  4,  6,  8, -1, -1, -1, -1],
  [ 2,  4,  6,  8, -1, -1, -1, -1],
  [ 0,  2,  4,  6,  8, -1, -1, -1],
  [10, -1, -1, -1, -1, -1, -1, -1],
  [ 0, 10, -1, -1, -1, -1, -1, -1],
  [ 2, 10, -1, -1, -1, -1, -1, -1],
  [ 0,  2, 10, -1, -1, -1, -1, -1],
  [ 4, 10, -1, -1, -1, -1, -1, -1],
  [ 0,  4, 10, -1, -1, -1, -1, -1],
  [ 2,  4, 10, -1, -1, -1, -1, -1],
  [ 0,  2,  4, 10, -1, -1, -1, -1],
  [ 6, 10, -1, -1, -1, -1, -1, -1],
  [ 0,  6, 10, -1, -1, -1, -1, -1],
  [ 2,  6, 10, -1, -1, -1, -1, -1],
  [ 0,  2,  6, 10, -1, -1, -1, -1],
  [ 4,  6, 10, -1, -1, -1, -1, -1],
  [ 0,  4,  6, 10, -1, -1, -1, -1],
  [ 2,  4,  6, 10, -1, -1, -1, -1],
  [ 0,  2,  4,  6, 10, -1, -1, -1],
  [ 8, 10, -1, -1, -1, -1, -1, -1],
  [ 0,  8, 10, -1, -1, -1, -1, -1],
  [ 2,  8, 10, -1, -1, -1, -1, -1],
  [ 0,  2,  8, 10, -1, -1, -1, -1],
  [ 4,  8, 10, -1, -1, -1, -1, -1],
  [ 0,  4,  8, 10, -1, -1, -1, -1],
  [ 2,  4,  8, 10, -1, -1, -1, -1],
  [ 0,  2,  4,  8, 10, -1, -1, -1],
  [ 6,  8, 10, -1, -1, -1, -1, -1],
  [ 0,  6,  8, 10, -1, -1, -1, -1],
  [ 2,  6,  8, 10, -1, -1, -1, -1],
  [ 0,  2,  6,  8, 10, -1, -1, -1],
  [ 4,  6,  8, 10, -1, -1, -1, -1],
  [ 0,  4,  6,  8, 10, -1, -1, -1],
  [ 2,  4,  6,  8, 10, -1, -1, -1],
  [ 0,  2,  4,  6,  8, 10, -1, -1],
  [12, -1, -1, -1, -1, -1, -1, -1],
  [ 0, 12, -1, -1, -1, -1, -1, -1],
  [ 2, 12, -1, -1, -1, -1, -1, -1],
  [ 0,  2, 12, -1, -1, -1, -1, -1],
  [ 4, 12, -1, -1, -1, -1, -1, -1],
  [ 0,  4, 12, -1, -1, -1, -1, -1],
  [ 2,  4, 12, -1, -1, -1, -1, -1],
  [ 0,  2,  4, 12, -1, -1, -1, -1],
  [ 6, 12, -1, -1, -1, -1, -1, -1],
  [ 0,  6, 12, -1, -1, -1, -1, -1],
  [ 2,  6, 12, -1, -1, -1, -1, -1],
  [ 0,  2,  6, 12, -1, -1, -1, -1],
  [ 4,  6, 12, -1, -1, -1, -1, -1],
  [ 0,  4,  6, 12, -1, -1, -1, -1],
  [ 2,  4,  6, 12, -1, -1, -1, -1],
  [ 0,  2,  4,  6, 12, -1, -1, -1],
  [ 8, 12, -1, -1, -1, -1, -1, -1],
  [ 0,  8, 12, -1, -1, -1, -1, -1],
  [ 2,  8, 12, -1, -1, -1, -1, -1],
  [ 0,  2,  8, 12, -1, -1, -1, -1],
  [ 4,  8, 12, -1, -1, -1, -1, -1],
  [ 0,  4,  8, 12, -1, -1, -1, -1],
  [ 2,  4,  8, 12, -1, -1, -1, -1],
  [ 0,  2,  4,  8, 12, -1, -1, -1],
  [ 6,  8, 12, -1, -1, -1, -1, -1],
  [ 0,  6,  8, 12, -1, -1, -1, -1],
  [ 2,  6,  8, 12, -1, -1, -1, -1],
  [ 0,  2,  6,  8, 12, -1, -1, -1],
  [ 4,  6,  8, 12, -1, -1, -1, -1],
  [ 0,  4,  6,  8, 12, -1, -1, -1],
  [ 2,  4,  6,  8, 12, -1, -1, -1],
  [ 0,  2,  4,  6,  8, 12, -1, -1],
  [10, 12, -1, -1, -1, -1, -1, -1],
  [ 0, 10, 12, -1, -1, -1, -1, -1],
  [ 2, 10, 12, -1, -1, -1, -1, -1],
  [ 0,  2, 10, 12, -1, -1, -1, -1],
  [ 4, 10, 12, -1, -1, -1, -1, -1],
  [ 0,  4, 10, 12, -1, -1, -1, -1],
  [ 2,  4, 10, 12, -1, -1, -1, -1],
  [ 0,  2,  4, 10, 12, -1, -1, -1],
  [ 6, 10, 12, -1, -1, -1, -1, -1],
  [ 0,  6, 10, 12, -1, -1, -1, -1],
  [ 2,  6, 10, 12, -1, -1, -1, -1],
  [ 0,  2,  6, 10, 12, -1, -1, -1],
  [ 4,  6, 10, 12, -1, -1, -1, -1],
  [ 0,  4,  6, 10, 12, -1, -1, -1],
  [ 2,  4,  6, 10, 12, -1, -1, -1],
  [ 0,  2,  4,  6, 10, 12, -1, -1],
  [ 8, 10, 12, -1, -1, -1, -1, -1],
  [ 0,  8, 10, 12, -1, -1, -1, -1],
  [ 2,  8, 10, 12, -1, -1, -1, -1],
  [ 0,  2,  8, 10, 12, -1, -1, -1],
  [ 4,  8, 10, 12, -1, -1, -1, -1],
  [ 0,  4,  8, 10, 12, -1, -1, -1],
  [ 2,  4,  8, 10, 12, -1, -1, -1],
  [ 0,  2,  4,  8, 10, 12, -1, -1],
  [ 6,  8, 10, 12, -1, -1, -1, -1],
  [ 0,  6,  8, 10, 12, -1, -1, -1],
  [ 2,  6,  8, 10, 12, -1, -1, -1],
  [ 0,  2,  6,  8, 10, 12, -1, -1],
  [ 4,  6,  8, 10, 12, -1, -1, -1],
  [ 0,  4,  6,  8, 10, 12, -1, -1],
  [ 2,  4,  6,  8, 10, 12, -1, -1],
  [ 0,  2,  4,  6,  8, 10, 12, -1],
  [14, -1, -1, -1, -1, -1, -1, -1],
  [ 0, 14, -1, -1, -1, -1, -1, -1],
  [ 2, 14, -1, -1, -1, -1, -1, -1],
  [ 0,  2, 14, -1, -1, -1, -1, -1],
  [ 4, 14, -1, -1, -1, -1, -1, -1],
  [ 0,  4, 14, -1, -1, -1, -1, -1],
  [ 2,  4, 14, -1, -1, -1, -1, -1],
  [ 0,  2,  4, 14, -1, -1, -1, -1],
  [ 6, 14, -1, -1, -1, -1, -1, -1],
  [ 0,  6, 14, -1, -1, -1, -1, -1],
  [ 2,  6, 14, -1, -1, -1, -1, -1],
  [ 0,  2,  6, 14, -1, -1, -1, -1],
  [ 4,  6, 14, -1, -1, -1, -1, -1],
  [ 0,  4,  6, 14, -1, -1, -1, -1],
  [ 2,  4,  6, 14, -1, -1, -1, -1],
  [ 0,  2,  4,  6, 14, -1, -1, -1],
  [ 8, 14, -1, -1, -1, -1, -1, -1],
  [ 0,  8, 14, -1, -1, -1, -1, -1],
  [ 2,  8, 14, -1, -1, -1, -1, -1],
  [ 0,  2,  8, 14, -1, -1, -1, -1],
  [ 4,  8, 14, -1, -1, -1, -1, -1],
  [ 0,  4,  8, 14, -1, -1, -1, -1],
  [ 2,  4,  8, 14, -1, -1, -1, -1],
  [ 0,  2,  4,  8, 14, -1, -1, -1],
  [ 6,  8, 14, -1, -1, -1, -1, -1],
  [ 0,  6,  8, 14, -1, -1, -1, -1],
  [ 2,  6,  8, 14, -1, -1, -1, -1],
  [ 0,  2,  6,  8, 14, -1, -1, -1],
  [ 4,  6,  8, 14, -1, -1, -1, -1],
  [ 0,  4,  6,  8, 14, -1, -1, -1],
  [ 2,  4,  6,  8, 14, -1, -1, -1],
  [ 0,  2,  4,  6,  8, 14, -1, -1],
  [10, 14, -1, -1, -1, -1, -1, -1],
  [ 0, 10, 14, -1, -1, -1, -1, -1],
  [ 2, 10, 14, -1, -1, -1, -1, -1],
  [ 0,  2, 10, 14, -1, -1, -1, -1],
  [ 4, 10, 14, -1, -1, -1, -1, -1],
  [ 0,  4, 10, 14, -1, -1, -1, -1],
  [ 2,  4, 10, 14, -1, -1, -1, -1],
  [ 0,  2,  4, 10, 14, -1, -1, -1],
  [ 6, 10, 14, -1, -1, -1, -1, -1],
  [ 0,  6, 10, 14, -1, -1, -1, -1],
  [ 2,  6, 10, 14, -1, -1, -1, -1],
  [ 0,  2,  6, 10, 14, -1, -1, -1],
  [ 4,  6, 10, 14, -1, -1, -1, -1],
  [ 0,  4,  6, 10, 14, -1, -1, -1],
  [ 2,  4,  6, 10, 14, -1, -1, -1],
  [ 0,  2,  4,  6, 10, 14, -1, -1],
  [ 8, 10, 14, -1, -1, -1, -1, -1],
  [ 0,  8, 10, 14, -1, -1, -1, -1],
  [ 2,  8, 10, 14, -1, -1, -1, -1],
  [ 0,  2,  8, 10, 14, -1, -1, -1],
  [ 4,  8, 10, 14, -1, -1, -1, -1],
  [ 0,  4,  8, 10, 14, -1, -1, -1],
  [ 2,  4,  8, 10, 14, -1, -1, -1],
  [ 0,  2,  4,  8, 10, 14, -1, -1],
  [ 6,  8, 10, 14, -1, -1, -1, -1],
  [ 0,  6,  8, 10, 14, -1, -1, -1],
  [ 2,  6,  8, 10, 14, -1, -1, -1],
  [ 0,  2,  6,  8, 10, 14, -1, -1],
  [ 4,  6,  8, 10, 14, -1, -1, -1],
  [ 0,  4,  6,  8, 10, 14, -1, -1],
  [ 2,  4,  6,  8, 10, 14, -1, -1],
  [ 0,  2,  4,  6,  8, 10, 14, -1],
  [12, 14, -1, -1, -1, -1, -1, -1],
  [ 0, 12, 14, -1, -1, -1, -1, -1],
  [ 2, 12, 14, -1, -1, -1, -1, -1],
  [ 0,  2, 12, 14, -1, -1, -1, -1],
  [ 4, 12, 14, -1, -1, -1, -1, -1],
  [ 0,  4, 12, 14, -1, -1, -1, -1],
  [ 2,  4, 12, 14, -1, -1, -1, -1],
  [ 0,  2,  4, 12, 14, -1, -1, -1],
  [ 6, 12, 14, -1, -1, -1, -1, -1],
  [ 0,  6, 12, 14, -1, -1, -1, -1],
  [ 2,  6, 12, 14, -1, -1, -1, -1],
  [ 0,  2,  6, 12, 14, -1, -1, -1],
  [ 4,  6, 12, 14, -1, -1, -1, -1],
  [ 0,  4,  6, 12, 14, -1, -1, -1],
  [ 2,  4,  6, 12, 14, -1, -1, -1],
  [ 0,  2,  4,  6, 12, 14, -1, -1],
  [ 8, 12, 14, -1, -1, -1, -1, -1],
  [ 0,  8, 12, 14, -1, -1, -1, -1],
  [ 2,  8, 12, 14, -1, -1, -1, -1],
  [ 0,  2,  8, 12, 14, -1, -1, -1],
  [ 4,  8, 12, 14, -1, -1, -1, -1],
  [ 0,  4,  8, 12, 14, -1, -1, -1],
  [ 2,  4,  8, 12, 14, -1, -1, -1],
  [ 0,  2,  4,  8, 12, 14, -1, -1],
  [ 6,  8, 12, 14, -1, -1, -1, -1],
  [ 0,  6,  8, 12, 14, -1, -1, -1],
  [ 2,  6,  8, 12, 14, -1, -1, -1],
  [ 0,  2,  6,  8, 12, 14, -1, -1],
  [ 4,  6,  8, 12, 14, -1, -1, -1],
  [ 0,  4,  6,  8, 12, 14, -1, -1],
  [ 2,  4,  6,  8, 12, 14, -1, -1],
  [ 0,  2,  4,  6,  8, 12, 14, -1],
  [10, 12, 14, -1, -1, -1, -1, -1],
  [ 0, 10, 12, 14, -1, -1, -1, -1],
  [ 2, 10, 12, 14, -1, -1, -1, -1],
  [ 0,  2, 10, 12, 14, -1, -1, -1],
  [ 4, 10, 12, 14, -1, -1, -1, -1],
  [ 0,  4, 10, 12, 14, -1, -1, -1],
  [ 2,  4, 10, 12, 14, -1, -1, -1],
  [ 0,  2,  4, 10, 12, 14, -1, -1],
  [ 6, 10, 12, 14, -1, -1, -1, -1],
  [ 0,  6, 10, 12, 14, -1, -1, -1],
  [ 2,  6, 10, 12, 14, -1, -1, -1],
  [ 0,  2,  6, 10, 12, 14, -1, -1],
  [ 4,  6, 10, 12, 14, -1, -1, -1],
  [ 0,  4,  6, 10, 12, 14, -1, -1],
  [ 2,  4,  6, 10, 12, 14, -1, -1],
  [ 0,  2,  4,  6, 10, 12, 14, -1],
  [ 8, 10, 12, 14, -1, -1, -1, -1],
  [ 0,  8, 10, 12, 14, -1, -1, -1],
  [ 2,  8, 10, 12, 14, -1, -1, -1],
  [ 0,  2,  8, 10, 12, 14, -1, -1],
  [ 4,  8, 10, 12, 14, -1, -1, -1],
  [ 0,  4,  8, 10, 12, 14, -1, -1],
  [ 2,  4,  8, 10, 12, 14, -1, -1],
  [ 0,  2,  4,  8, 10, 12, 14, -1],
  [ 6,  8, 10, 12, 14, -1, -1, -1],
  [ 0,  6,  8, 10, 12, 14, -1, -1],
  [ 2,  6,  8, 10, 12, 14, -1, -1],
  [ 0,  2,  6,  8, 10, 12, 14, -1],
  [ 4,  6,  8, 10, 12, 14, -1, -1],
  [ 0,  4,  6,  8, 10, 12, 14, -1],
  [ 2,  4,  6,  8, 10, 12, 14, -1],
  [ 0,  2,  4,  6,  8, 10, 12, 14]
];
