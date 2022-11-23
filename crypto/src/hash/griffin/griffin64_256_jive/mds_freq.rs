// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source &code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

// FFT-BASED MDS MULTIPLICATION HELPER FUNCTIONS
// ================================================================================================

/// This module contains helper functions as well as constants used to perform the vector-matrix
/// multiplication step of the Griffin permutation. The special form of our MDS matrix
/// i.e. being circular, allows us to reduce the vector-matrix multiplication to a Hadamard product
/// of two vectors in "frequency domain". This follows from the simple fact that every circulant
/// matrix has the columns of the discrete Fourier transform matrix as orthogonal eigenvectors.
/// The implementation also avoids the use of internal 2-point FFTs, and 2-point iFFTs, and substitutes
/// them with explicit expressions. It also avoids, due to the form of our matrix in the frequency domain,
/// divisions by 2 and repeated modular reductions. This is because of our explicit choice of
/// an MDS matrix that has small powers of 2 entries in frequency domain.
/// The following implementation has benefited greatly from the discussions and insights of
/// Hamish Ivey-Law and Jacqueline Nabaglo of Polygon Zero.

// Griffin MDS matrix in frequency domain.
// More precisely, this is the output of the two 4-point (real) FFTs of the first column of
// the MDS matrix i.e. just before the multiplication with the appropriate twiddle factors
// and application of the final four 2-point FFT in order to get the full 8-point FFT.
// The entries have been scaled appropriately in order to avoid divisions by 2 in iFFT2 and iFFT4.
const MDS_FREQ_BLOCK_ONE: [i64; 2] = [16, 8];
const MDS_FREQ_BLOCK_TWO: [(i64, i64); 2] = [(8, -4), (-1, 1)];
const MDS_FREQ_BLOCK_THREE: [i64; 2] = [-1, 1];

// We use split 2 x 4 FFT transform in order to transform our vectors into the frequency domain.
#[inline(always)]
pub(crate) fn mds_multiply_freq(state: [u64; 8]) -> [u64; 8] {
    let [s0, s1, s2, s3, s4, s5, s6, s7] = state;

    let (u0, u1, u2) = fft4_real([s0, s2, s4, s6]);
    let (u4, u5, u6) = fft4_real([s1, s3, s5, s7]);

    let [v0, v4] = block1([u0, u4], MDS_FREQ_BLOCK_ONE);
    let [v1, v5] = block2([u1, u5], MDS_FREQ_BLOCK_TWO);
    let [v2, v6] = block3([u2, u6], MDS_FREQ_BLOCK_THREE);
    // The 4th block is not computed as it is similar to the 2nd one, up to complex conjugation,
    // and is, due to the use of the real FFT and iFFT, redundant.

    let [s0, s2, s4, s6] = ifft4_real((v0, v1, v2));
    let [s1, s3, s5, s7] = ifft4_real((v4, v5, v6));

    [s0, s1, s2, s3, s4, s5, s6, s7]
}

// We use the real FFT to avoid redundant computations. See https://www.mdpi.com/2076-3417/12/9/4700
#[inline(always)]
fn fft2_real(x: [u64; 2]) -> [i64; 2] {
    [(x[0] as i64 + x[1] as i64), (x[0] as i64 - x[1] as i64)]
}

#[inline(always)]
fn ifft2_real(y: [i64; 2]) -> [u64; 2] {
    // We avoid divisions by 2 by appropriately scaling the MDS matrix constants.
    [(y[0] + y[1]) as u64, (y[0] - y[1]) as u64]
}

#[inline(always)]
fn fft4_real(x: [u64; 4]) -> (i64, (i64, i64), i64) {
    let [z0, z2] = fft2_real([x[0], x[2]]);
    let [z1, z3] = fft2_real([x[1], x[3]]);
    let y0 = z0 + z1;
    let y1 = (z2, -z3);
    let y2 = z0 - z1;
    (y0, y1, y2)
}

#[inline(always)]
fn ifft4_real(y: (i64, (i64, i64), i64)) -> [u64; 4] {
    // In calculating 'z0' and 'z1', division by 2 is avoided by appropriately scaling
    // the MDS matrix constants.
    let z0 = y.0 + y.2;
    let z1 = y.0 - y.2;
    let z2 = y.1 .0;
    let z3 = -y.1 .1;

    let [x0, x2] = ifft2_real([z0, z2]);
    let [x1, x3] = ifft2_real([z1, z3]);

    [x0, x1, x2, x3]
}

#[inline(always)]
fn block1(x: [i64; 2], y: [i64; 2]) -> [i64; 2] {
    let [x0, x1] = x;
    let [y0, y1] = y;
    let z0 = x0 * y0 + x1 * y1;
    let z1 = x0 * y1 + x1 * y0;

    [z0, z1]
}

#[inline(always)]
fn block2(x: [(i64, i64); 2], y: [(i64, i64); 2]) -> [(i64, i64); 2] {
    let [(x0r, x0i), (x1r, x1i)] = x;
    let [(y0r, y0i), (y1r, y1i)] = y;
    let x0s = x0r + x0i;
    let x1s = x1r + x1i;
    let y0s = y0r + y0i;
    let y1s = y1r + y1i;

    // Compute x0​y0 ​− ix1​y1​ using Karatsuba for complex numbers multiplication
    let m0 = (x0r * y0r, x0i * y0i);
    let m1 = (x1r * y1r, x1i * y1i);
    let z0r = (m0.0 - m0.1) + (x1s * y1s - m1.0 - m1.1);
    let z0i = (x0s * y0s - m0.0 - m0.1) + (-m1.0 + m1.1);
    let z0 = (z0r, z0i);

    // Compute x0​y1​ + x1​y0 using Karatsuba for complex numbers multiplication
    let m0 = (x0r * y1r, x0i * y1i);
    let m1 = (x1r * y0r, x1i * y0i);
    let z1r = (m0.0 - m0.1) + (m1.0 - m1.1);
    let z1i = (x0s * y1s - m0.0 - m0.1) + (x1s * y0s - m1.0 - m1.1);
    let z1 = (z1r, z1i);

    [z0, z1]
}

#[inline(always)]
fn block3(x: [i64; 2], y: [i64; 2]) -> [i64; 2] {
    let [x0, x1] = x;
    let [y0, y1] = y;
    let z0 = x0 * y0 - x1 * y1;
    let z1 = x0 * y1 + x1 * y0;

    [z0, z1]
}

// TESTS
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::super::GriffinJive64_256;
    use crate::hash::griffin::griffin64_256_jive::MDS;
    use math::{fields::f64::BaseElement, FieldElement};
    use proptest::prelude::*;

    const STATE_WIDTH: usize = 8;

    #[inline(always)]
    fn apply_mds_naive(state: &mut [BaseElement; STATE_WIDTH]) {
        let mut result = [BaseElement::ZERO; STATE_WIDTH];
        result.iter_mut().zip(MDS).for_each(|(r, mds_row)| {
            state.iter().zip(mds_row).for_each(|(&s, m)| {
                *r += m * s;
            });
        });
        *state = result;
    }

    proptest! {
        #[test]
        fn mds_freq_proptest(a in any::<[u64;STATE_WIDTH]>()) {

            let mut v1 = [BaseElement::ZERO;STATE_WIDTH];
            let mut v2;

            for i in 0..STATE_WIDTH {
                v1[i] = BaseElement::new(a[i]);
            }
            v2 = v1.clone();

            apply_mds_naive(&mut v1);
            GriffinJive64_256::apply_linear(&mut v2);

            prop_assert_eq!(v1, v2);
        }
    }
}
