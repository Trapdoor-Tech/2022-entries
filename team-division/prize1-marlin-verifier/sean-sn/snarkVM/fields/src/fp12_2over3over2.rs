// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use core::ffi::c_void;
use crate::{fp6_3over2::*, Field, Fp2, Fp2Parameters, One, Zero};
use snarkvm_utilities::{bititerator::BitIteratorBE, rand::Uniform, serialize::*, FromBytes, ToBits, ToBytes};

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    io::{Read, Result as IoResult, Write},
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

pub trait Fp12Parameters: 'static + Send + Sync + Copy {
    type Fp6Params: Fp6Parameters;

    /// Coefficients for the Frobenius automorphism.
    const FROBENIUS_COEFF_FP12_C1: [Fp2<Fp2Params<Self>>; 12];
}

/// An element of Fp12, represented by c0 + c1 * v
#[derive(Derivative, Serialize, Deserialize)]
#[derivative(
    Default(bound = "P: Fp12Parameters"),
    Hash(bound = "P: Fp12Parameters"),
    Clone(bound = "P: Fp12Parameters"),
    Copy(bound = "P: Fp12Parameters"),
    Debug(bound = "P: Fp12Parameters"),
    PartialEq(bound = "P: Fp12Parameters"),
    Eq(bound = "P: Fp12Parameters")
)]
#[must_use]
pub struct Fp12<P: Fp12Parameters> {
    pub c0: Fp6<P::Fp6Params>,
    pub c1: Fp6<P::Fp6Params>,
}

extern "C" {
    fn blst_fp12_377_mul(ret: *mut c_void, a: *const c_void, b: *const c_void);
    fn blst_fp12_377_sqr(ret: *mut c_void, a: *const c_void);
    fn blst_fp12_377_cyclotomic_sqr(ret: *mut c_void, a: *const c_void);
    fn blst_fp12_377_inverse(ret: *mut c_void, a: *const c_void);
    fn blst_fp12_377_conjugate(ret: *mut c_void);
    fn blst_fp12_377_frobenius_map(ret: *mut c_void, a: *const c_void, n: usize);
    fn blst_fp12_377_is_one(a: *const c_void) -> bool;
}

type Fp2Params<P> = <<P as Fp12Parameters>::Fp6Params as Fp6Parameters>::Fp2Params;

impl<P: Fp12Parameters> Fp12<P> {
    /// Multiply by quadratic nonresidue v.
    #[inline(always)]
    pub(crate) fn mul_fp6_by_nonresidue(fe: &Fp6<P::Fp6Params>) -> Fp6<P::Fp6Params> {
        let new_c0 = P::Fp6Params::mul_fp2_by_nonresidue(&fe.c2);
        let new_c1 = fe.c0;
        let new_c2 = fe.c1;
        Fp6::new(new_c0, new_c1, new_c2)
    }

    pub fn new(c0: Fp6<P::Fp6Params>, c1: Fp6<P::Fp6Params>) -> Self {
        Self { c0, c1 }
    }

    pub fn mul_by_fp(&mut self, element: &<<P::Fp6Params as Fp6Parameters>::Fp2Params as Fp2Parameters>::Fp) {
        self.c0.mul_by_fp(element);
        self.c1.mul_by_fp(element);
    }

    pub fn conjugate(&mut self) {
        unsafe { blst_fp12_377_conjugate(self as *mut Self as *mut c_void) };
    }

    pub fn mul_by_034(&mut self, c0: &Fp2<Fp2Params<P>>, c3: &Fp2<Fp2Params<P>>, c4: &Fp2<Fp2Params<P>>) {
        let a0 = self.c0.c0 * c0;
        let a1 = self.c0.c1 * c0;
        let a2 = self.c0.c2 * c0;
        let a = Fp6::new(a0, a1, a2);
        let mut b = self.c1;
        b.mul_by_01(c3, c4);

        let c0 = *c0 + c3;
        let c1 = c4;
        let mut e = self.c0 + self.c1;
        e.mul_by_01(&c0, c1);
        self.c1 = e - (a + b);
        self.c0 = a + Self::mul_fp6_by_nonresidue(&b);
    }

    pub fn mul_by_014(&mut self, c0: &Fp2<Fp2Params<P>>, c1: &Fp2<Fp2Params<P>>, c4: &Fp2<Fp2Params<P>>) {
        let mut aa = self.c0;
        aa.mul_by_01(c0, c1);
        let mut bb = self.c1;
        bb.mul_by_1(c4);
        let mut o = *c1;
        o.add_assign(c4);
        self.c1.add_assign(self.c0);
        self.c1.mul_by_01(c0, &o);
        self.c1.sub_assign(&aa);
        self.c1.sub_assign(&bb);
        self.c0 = bb;
        self.c0 = Self::mul_fp6_by_nonresidue(&self.c0);
        self.c0.add_assign(aa);
    }

    pub fn cyclotomic_square(&self) -> Self {
        let mut result = Self::zero();
        unsafe {
            blst_fp12_377_cyclotomic_sqr(
                &mut result as *mut Self as *mut c_void,
                self as *const Self as *const c_void,
            )
        };
        result
    }

    pub fn cyclotomic_exp<S: AsRef<[u64]>>(&self, exp: S) -> Self {
        let mut res = Self::one();

        let mut found_one = false;

        for i in BitIteratorBE::new(exp) {
            if !found_one {
                if i {
                    found_one = true;
                } else {
                    continue;
                }
            }

            res = res.cyclotomic_square();

            if i {
                res *= self;
            }
        }
        res
    }
}

impl<P: Fp12Parameters> std::fmt::Display for Fp12<P> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(f, "Fp12({} + {} * w)", self.c0, self.c1)
    }
}

impl<P: Fp12Parameters> Distribution<Fp12<P>> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp12<P> {
        Fp12::new(Uniform::rand(rng), Uniform::rand(rng))
    }
}

impl<P: Fp12Parameters> Zero for Fp12<P> {
    fn zero() -> Self {
        Self::new(Fp6::zero(), Fp6::zero())
    }

    fn is_zero(&self) -> bool {
        self.c0.is_zero() && self.c1.is_zero()
    }
}

impl<P: Fp12Parameters> One for Fp12<P> {
    fn one() -> Self {
        Self::new(Fp6::one(), Fp6::zero())
    }

    fn is_one(&self) -> bool {
        unsafe { blst_fp12_377_is_one(self as *const Self as *const c_void) }
    }
}

impl<P: Fp12Parameters> Field for Fp12<P> {
    type BasePrimeField = <Fp6<P::Fp6Params> as Field>::BasePrimeField;

    fn from_base_prime_field(other: Self::BasePrimeField) -> Self {
        Self::new(Fp6::from_base_prime_field(other), Fp6::zero())
    }

    #[inline]
    fn characteristic<'a>() -> &'a [u64] {
        Fp6::<P::Fp6Params>::characteristic()
    }

    fn double(&self) -> Self {
        let mut copy = *self;
        copy.double_in_place();
        copy
    }

    #[inline]
    fn from_random_bytes_with_flags<F: Flags>(bytes: &[u8]) -> Option<(Self, F)> {
        let split_at = bytes.len() / 2;
        if let Some(c0) = Fp6::<P::Fp6Params>::from_random_bytes(&bytes[..split_at]) {
            if let Some((c1, flags)) = Fp6::<P::Fp6Params>::from_random_bytes_with_flags::<F>(&bytes[split_at..]) {
                return Some((Fp12::new(c0, c1), flags));
            }
        }
        None
    }

    #[inline]
    fn from_random_bytes(bytes: &[u8]) -> Option<Self> {
        Self::from_random_bytes_with_flags::<EmptyFlags>(bytes).map(|f| f.0)
    }

    fn double_in_place(&mut self) {
        self.c0.double_in_place();
        self.c1.double_in_place();
    }

    fn frobenius_map(&mut self, power: usize) {
        if power > 0 && power <= 3 {
            unsafe {
                blst_fp12_377_frobenius_map(
                    self as *mut Self as *mut c_void,
                    self as *const Self as *const c_void,
                    power,
                )
            };
            return;
        }
        self.c0.frobenius_map(power);
        self.c1.frobenius_map(power);

        self.c1.c0.mul_assign(&P::FROBENIUS_COEFF_FP12_C1[power % 12]);
        self.c1.c1.mul_assign(&P::FROBENIUS_COEFF_FP12_C1[power % 12]);
        self.c1.c2.mul_assign(&P::FROBENIUS_COEFF_FP12_C1[power % 12]);
    }

    fn square(&self) -> Self {
        let mut result = Self::zero();
        unsafe {
            blst_fp12_377_sqr(
                &mut result as *mut Self as *mut c_void,
                self as *const Self as *const c_void,
            )
        };
        result
    }

    fn square_in_place(&mut self) -> &mut Self {
        unsafe {
            blst_fp12_377_sqr(
                self as *mut Self as *mut c_void,
                self as *const Self as *const c_void,
            )
        };
        self
    }

    fn inverse(&self) -> Option<Self> {
        if self.is_zero() {
            None
        } else {
            let mut result = Self::zero();
            unsafe {
                blst_fp12_377_inverse(
                    &mut result as *mut Self as *mut c_void,
                    self as *const Self as *const c_void,
                )
            };
            Some(result)
        }
    }

    fn inverse_in_place(&mut self) -> Option<&mut Self> {
        match self.inverse() {
            Some(inv) => {
                *self = inv;
                Some(self)
            }
            None => None,
        }
    }
}

impl<P: Fp12Parameters> Neg for Fp12<P> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        let mut copy = Self::zero();
        copy.c0 = self.c0.neg();
        copy.c1 = self.c1.neg();
        copy
    }
}

impl_add_sub_from_field_ref!(Fp12, Fp12Parameters);
impl_mul_div_from_field_ref!(Fp12, Fp12Parameters);

impl<'a, P: Fp12Parameters> Add<&'a Self> for Fp12<P> {
    type Output = Self;

    #[inline]
    fn add(self, other: &Self) -> Self {
        let mut result = self;
        result.add_assign(other);
        result
    }
}

impl<'a, P: Fp12Parameters> Sub<&'a Self> for Fp12<P> {
    type Output = Self;

    #[inline]
    fn sub(self, other: &Self) -> Self {
        let mut result = self;
        result.sub_assign(&other);
        result
    }
}

impl<'a, P: Fp12Parameters> Mul<&'a Self> for Fp12<P> {
    type Output = Self;

    #[inline]
    fn mul(self, other: &Self) -> Self {
        let mut result = Self::zero();
        unsafe {
            blst_fp12_377_mul(
                &mut result as *mut Self as *mut c_void,
                &self as *const Self as *const c_void,
                other as *const Self as *const c_void,
            )
        };
        result
    }
}

impl<'a, P: Fp12Parameters> Div<&'a Self> for Fp12<P> {
    type Output = Self;

    #[inline]
    fn div(self, other: &Self) -> Self {
        let mut result = self;
        result.mul_assign(&other.inverse().unwrap());
        result
    }
}

impl<'a, P: Fp12Parameters> AddAssign<&'a Self> for Fp12<P> {
    #[inline]
    fn add_assign(&mut self, other: &Self) {
        self.c0.add_assign(other.c0);
        self.c1.add_assign(other.c1);
    }
}

impl<'a, P: Fp12Parameters> SubAssign<&'a Self> for Fp12<P> {
    #[inline]
    fn sub_assign(&mut self, other: &Self) {
        self.c0.sub_assign(&other.c0);
        self.c1.sub_assign(&other.c1);
    }
}

impl<'a, P: Fp12Parameters> MulAssign<&'a Self> for Fp12<P> {
    #[inline]
    #[allow(clippy::suspicious_op_assign_impl)]
    fn mul_assign(&mut self, other: &Self) {
        unsafe {
            blst_fp12_377_mul(
                self as *mut Self as *mut c_void,
                self as *const Self as *const c_void,
                other as *const Self as *const c_void,
            )
        };
    }
}

impl<'a, P: Fp12Parameters> DivAssign<&'a Self> for Fp12<P> {
    #[inline]
    fn div_assign(&mut self, other: &Self) {
        self.mul_assign(&other.inverse().unwrap());
    }
}

impl<P: Fp12Parameters> Ord for Fp12<P> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        let c1_cmp = self.c1.cmp(&other.c1);
        if c1_cmp == Ordering::Equal { self.c0.cmp(&other.c0) } else { c1_cmp }
    }
}

impl<P: Fp12Parameters> PartialOrd for Fp12<P> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<P: Fp12Parameters> From<u128> for Fp12<P> {
    fn from(other: u128) -> Self {
        Self::new(other.into(), Fp6::zero())
    }
}

impl<P: Fp12Parameters> From<u64> for Fp12<P> {
    fn from(other: u64) -> Self {
        Self::new(other.into(), Fp6::zero())
    }
}

impl<P: Fp12Parameters> From<u32> for Fp12<P> {
    fn from(other: u32) -> Self {
        Self::new(other.into(), Fp6::zero())
    }
}

impl<P: Fp12Parameters> From<u16> for Fp12<P> {
    fn from(other: u16) -> Self {
        Self::new(other.into(), Fp6::zero())
    }
}

impl<P: Fp12Parameters> From<u8> for Fp12<P> {
    fn from(other: u8) -> Self {
        Self::new(other.into(), Fp6::zero())
    }
}

impl<P: Fp12Parameters> ToBits for Fp12<P> {
    fn to_bits_le(&self) -> Vec<bool> {
        let mut res = vec![];
        res.extend_from_slice(&self.c0.to_bits_le());
        res.extend_from_slice(&self.c1.to_bits_le());
        res
    }

    fn to_bits_be(&self) -> Vec<bool> {
        let mut res = vec![];
        res.extend_from_slice(&self.c0.to_bits_be());
        res.extend_from_slice(&self.c1.to_bits_be());
        res
    }
}

impl<P: Fp12Parameters> ToBytes for Fp12<P> {
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.c0.write_le(&mut writer)?;
        self.c1.write_le(&mut writer)
    }
}

impl<P: Fp12Parameters> FromBytes for Fp12<P> {
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        let c0 = Fp6::read_le(&mut reader)?;
        let c1 = Fp6::read_le(&mut reader)?;
        Ok(Fp12::new(c0, c1))
    }
}

impl<P: Fp12Parameters> CanonicalSerializeWithFlags for Fp12<P> {
    #[inline]
    fn serialize_with_flags<W: Write, F: Flags>(&self, mut writer: W, flags: F) -> Result<(), SerializationError> {
        self.c0.serialize_uncompressed(&mut writer)?;
        self.c1.serialize_with_flags(&mut writer, flags)?;
        Ok(())
    }

    fn serialized_size_with_flags<F: Flags>(&self) -> usize {
        self.c0.uncompressed_size() + self.c1.serialized_size_with_flags::<F>()
    }
}

impl<P: Fp12Parameters> CanonicalSerialize for Fp12<P> {
    #[inline]
    fn serialize_with_mode<W: Write>(&self, writer: W, _compress: Compress) -> Result<(), SerializationError> {
        self.serialize_with_flags(writer, EmptyFlags)
    }

    #[inline]
    fn serialized_size(&self, compress: Compress) -> usize {
        self.c0.serialized_size(compress) + self.c1.serialized_size(compress)
    }
}

impl<P: Fp12Parameters> CanonicalDeserializeWithFlags for Fp12<P> {
    #[inline]
    fn deserialize_with_flags<R: Read, F: Flags>(mut reader: R) -> Result<(Self, F), SerializationError> {
        let c0 = CanonicalDeserialize::deserialize_uncompressed(&mut reader)?;
        let (c1, flags) = Fp6::deserialize_with_flags(&mut reader)?;
        Ok((Self::new(c0, c1), flags))
    }
}

impl<P: Fp12Parameters> Valid for Fp12<P> {
    fn check(&self) -> Result<(), snarkvm_utilities::SerializationError> {
        Ok(())
    }

    fn batch_check<'a>(_batch: impl Iterator<Item = &'a Self>) -> Result<(), snarkvm_utilities::SerializationError>
    where
        Self: 'a,
    {
        Ok(())
    }
}

impl<P: Fp12Parameters> CanonicalDeserialize for Fp12<P> {
    #[inline]
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Self, SerializationError> {
        let c0 = CanonicalDeserialize::deserialize_with_mode(&mut reader, compress, validate)?;
        let c1 = CanonicalDeserialize::deserialize_with_mode(&mut reader, compress, validate)?;
        Ok(Fp12::new(c0, c1))
    }
}
