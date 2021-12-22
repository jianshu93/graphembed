//! implements a randomized generalized svd building upon
//! randomized svd.
//! We implement algorithm 2.4 from : 
//!     *Randomized General Singular Value Decomposition CAMC 2021*
//!     W. Wei, H. Zhang, X. Yang, X. Chen
//! We build upon the crate annembed which give us algo 2.3 of the same paper
//! (which corresponds to algo 4.2 of Halko-Tropp)

#![allow(unused)]
// num_traits::float::Float : Num + Copy + NumCast + PartialOrd + Neg<Output = Self>,  PartialOrd which is not in Scalar.
//     and nan() etc

// num_traits::Real : Num + Copy + NumCast + PartialOrd + Neg<Output = Self>
// as float but without nan() infinite() 

// ndarray::ScalarOperand provides array * F
// ndarray_linalg::Scalar provides Exp notation + Display + Debug + Serialize and sum on iterators

use anyhow::{Error, anyhow};

use num_traits::float::*;    // tp get FRAC_1_PI from FloatConst
use num_traits::cast::FromPrimitive;


use ndarray::{Array1, Array2, ViewRepr, Ix1, Ix2};

use ndarray_linalg::{Scalar, Lapack};
use std::any::TypeId;

use lapacke::{cggsvd3, Layout};

// this module provides svdapproximation tools à la Hlako-Tropp
use annembed::tools::svdapprox::*;



// We fist implement a Range approximation with a precision criteria as 
// this one can be done with Sparse Matrix. Moreover it help determine the rank


#[cfg_attr(doc, katexit::katexit)]
/// We searh a generalized svd for the pair of matrix mat_1 (m,n) and mat_2 (p,n)
/// i.e we search 2 orthogonal matrices $V_1$ and $V_2$ , 2 diagonal matrices $\Sigma_{1}$ and $\Sigma_{1}$
/// and one non singular matrix X such that:
/// $$ V_{1}^{t} * mat1 * X = \Sigma_{1} \space and \space
/// 
///     V_{2}^{t} * mat2 * X = \Sigma_{2} $$
///  
/// The optional parameters can be used to modify (by multiplication or transposition) the 2 matrices mat1 and mat2.  
/// This avoids some matrix reallocations befote entering lapack.  
/// They are described in the GSvdOptParams documentation.
/// 
/// Most often the matrix representation will be CSR and the precision approximation mode will
/// be used. But for small graph we can consider the approximation with given target rank
/// 
pub struct GSvdApprox<'a, F: Scalar> {
    /// first matrix we want to approximate range of
    mat1 : &'a MatRepr<F>,
    /// second matrix
    mat2 : &'a MatRepr<F>,
    /// optional parameters
    opt_params : Option<GSvdOptParams>,
    /// approximation mode
    precision : RangeApproxMode,
}   // end of struct GsvdApprox


#[derive(Copy, Clone, Debug)]
/// This structure describes optionam parameters used to specify the Gsvd approximation to do by GSvdApprox
/// It can be useful to keep the two matrices mat1 and mat2 stored in GSvdApprox in one order but to solve the problem for their transpose
/// (as is the case in the Hope algorithm).  
/// In this case the transpose flags are used to send to lapack the matrices with a transpose flag.
/// For the multplication factor (also useful in the Hope algorithm they are applied in a later stage of the algorithm) 
pub struct GSvdOptParams {
    /// multiplication factor to use for mat1. default to 1.
    alpha_1 : f64, 
    /// transposition to apply to mat1. default to no
    transpose_1 : bool,
    /// multiplication factor to use for mat2. default to 1.
    alpha_2 : f64, 
    /// transposition to apply to mat2? default to no
    transpose_2 : bool,    
}  // end of struct GSvdOptParams


impl GSvdOptParams {
    pub fn new(alpha_1 : f64,  transpose_1 : bool,  alpha_2 : f64 , transpose_2 : bool) -> Self {
        GSvdOptParams {alpha_1, transpose_1, alpha_2, transpose_2}   
    } // end of new GSvdOptParams

    pub fn get_alpha_1(&self) -> f64 { self. alpha_1}

    pub fn get_alpha_2(&self) -> f64 { self. alpha_2}

    pub fn get_transpose_1(&self) -> bool { self.transpose_1}

    pub fn get_transpose_2(&self) -> bool { self.transpose_2}

} // end of impl GSvdOptParams


#[cfg_attr(doc, katexit::katexit)]
/// For a problem described in GSvdApprox by the pair of matrix mat_1 (m,n) and mat_2 (p,n)
/// we get:  
/// 
///  - 2 orthogonal matrices  $V_{1}$  and  $V_{2}$
///       
///  - 2 diagonal matrices $\Sigma_{1}$ and $\Sigma_{1}$  
/// 
///  - one non singular matrix X such that:
/// $$ V_{1}^{t} * mat1 * X = \Sigma_{1} \space and \space
///    V_{2}^{t} * mat2 * X = \Sigma_{2} $$
/// 
pub struct GSvdResult<F> {
    /// eigenvalues
    pub(crate)  v1 : Option<Array2<F>>,
    /// left eigenvectors. (m,r) matrix where r is rank asked for and m the number of data.
    pub(crate)  v2 : Option<Array2<F>>,
    /// first (diagonal matrix) eigenvalues
    pub(crate)  s1 : Option<Array1<F>>,
    /// second (diagonal matrix) eigenvalues
    pub(crate)  s2 : Option<Array1<F>>,
    /// common right term of mat1 and mat2 factorization
    pub(crate) commonx : Option<Array2<F>>
} // end of struct SvdResult<F> 


impl <F> GSvdResult<F> {

    pub(crate) fn new() -> Self {
        GSvdResult{v1 :None, v2 : None, s1 : None, s2 : None, commonx :None}
    }

    // reconstruct result from the out parameters of lapack. For us u and v are always asked for
    pub(crate) fn init_from_lapack(&mut self, u : Array2<F>, v : Array2<F>, k : i32 ,l : i32 , alpha : Array1<F>, beta : Array1<F>) {
        panic!("not yet implemented");
    }
} // end of impl block for GSvdResult



impl  <'a, F> GSvdApprox<'a, F>  
    where  F : Float + Lapack + Scalar  + ndarray::ScalarOperand + sprs::MulAcc {
    /// We impose the RangePrecision mode for now.
    pub fn new(mat1 : &'a MatRepr<F>, mat2 : &'a MatRepr<F>, precision : RangePrecision, opt_params : Option<GSvdOptParams>) -> Self {
        // TODO check for dimensions constraints, and type representation

        return GSvdApprox{mat1, mat2, opt_params, precision : RangeApproxMode::EPSIL(precision)};
    } // end of new

    /// return optional paramertes if any
    pub fn get_parameters(&mut self,  alpha_1 : f64,  transpose_1 : bool,  alpha_2 : f64 , transpose_2 : bool) -> &Option<GSvdOptParams> {
        &self.opt_params
    } // end of set_parameters


    // We have to :
    //   - do a range approximation of the 2 matrices in problem definition
    //   - do a (full) gsvd of the 2 reduced matrices 
    //   - lapck rust interface requires we pass matrix as slices so they must be in row order!
    //     but for our application we must pass transposed version of Mg and Ml as we must compute inverse(Mg) * Ml
    //     with a = Mg and b = Ml. So it seems we cannot avoid copying when construction the GSvdApprox

    /// 
    pub fn do_approx_gsvd(&self) -> Result<GSvdResult<F>, anyhow::Error> {
        // We construct an approximation first for mat1 and then for mat2 and with the same precision 
        // criterion
        let r_approx1 = RangeApprox::new(self.mat1, self.precision);
        let  approx1_res = r_approx1.get_approximator();
        if approx1_res.is_none() {
            return Err(anyhow!("approximation of matrix 1 failed"));
        }
        let approx1_res = approx1_res.unwrap();
        let r_approx2 = RangeApprox::new(self.mat2, self.precision);
        let  approx2_res = r_approx2.get_approximator();
        if approx2_res.is_none() {
            return Err(anyhow!("approximation of matrix 2 failed"));
        }
        let approx2_res = approx2_res.unwrap();
        // We must not check for the ranks of approx1_res and approx2_res.
        // We want the 2 matrix to have the same weights but if we ran in precision mode we must
        // enforce that.
        // With Halko-Tropp (or Wei-Zhang and al.) conventions, we have mat1 = (m1,n), mat2 = (m2,n)
        // we get  approx1_res = (m1, l1)  and (m2, l2).
        // We must now construct reduced matrix approximating mat1 and mat2 i.e t(approx1_res)* mat1 
        // and t(approx2_res)* mat2 and get matrices (l1,n) and (l2,n)
        let mut a = match self.mat1.get_data() {
            MatMode::FULL(mat) => { approx1_res.t().dot(mat)},
            MatMode::CSR(mat)  => { 
                                    log::trace!("direct_svd got csr matrix");
                                    small_transpose_dense_mult_csr(&approx1_res, mat)
                                },
        };
        let mut b = match self.mat2.get_data() {
            MatMode::FULL(mat) => { approx2_res.t().dot(mat)},
            MatMode::CSR(mat)  => { 
                                    log::trace!("direct_svd got csr matrix");
                                    small_transpose_dense_mult_csr(&approx2_res, mat)
                                },
        };
        // now we must do the standard generalized svd (with Lapack ggsvd3) for m and reduced_n
        // We are at step iv) of algo 2.4 of Wei and al.
        // See rust doc https://docs.rs/lapacke/latest/lapacke/fn.cggsvd3_work.html and
        // fortran https://www.netlib.org/lapack/lug/node36.html#1815 but new function is (s|d)ggsvd3
        //
        // Lapack definition of GSVD is in the following link:
        // http://www.netlib.org/lapack/explore-html/d1/d7e/group__double_g_esing_gab6c743f531c1b87922eb811cbc3ef645.html
        //
        //  Lapack GSVD(A,B) for A=(m,n) and B=(p,n) 
        //  gives U**T*A*Q = D1*( 0 R ),    V**T*B*Q = D2*( 0 R )   with  U , V and Q orthogonals
        //
        let (a_nbrow, a_nbcol) = a.dim();
        let jobu = b'U';
        let jobv = b'V';
        let jobq = b'N'; // Q is large we do not need it, we do not compute it
        assert_eq!(a_nbcol, b.dim().1); // check m and n have the same number of columns.
        let mut k : i32 = 0;
        let mut l : i32 = 0;
        // TODO check lda ...
        // Caution our matrix are C (row) ordered so lda si 1. but we want to send the transpose (!) so lda is a_nbrow
        let lda : i32 = a_nbcol as i32;
        let b_dim = b.dim();
        // caution our matrix are C (row) ordered so lda si 1. but we want to send the transpose (!) so lda is a_nbrow
        let ldb : i32 = b_dim.0 as i32;
        let ires: i32;
        let ldu = a_nbrow;  // as we compute U , ldu must be greater than nb rows of A
        let ldu = a_nbrow as i32;
        let ldv = a_nbrow as i32;
        //
        let ldq = 0;
        let mut iwork = Vec::<i32>::with_capacity(a_nbcol);
        let u : Array2::<F>;
        let v : Array2::<F>;
        let alpha : Array1::<F>;
        let beta : Array1::<F>;
        let mut gsvdres = GSvdResult::<F>::new();
        //
        if TypeId::of::<F>() == TypeId::of::<f32>() {
            let mut alpha_f32 = Vec::<f32>::with_capacity(a_nbcol);
            let mut beta_f32 = Vec::<f32>::with_capacity(a_nbcol);
            let mut u_f32= Array2::<f32>::zeros((a_nbrow, a_nbrow));
            let mut v_f32= Array2::<f32>::zeros((b_dim.0, b_dim.0));
            let mut q_f32 = Vec::<f32>::new();
            ires = unsafe {
                // we must cast a and b to f32 slices!! unsafe but we know our types with TypeId
                let mut af32 = std::slice::from_raw_parts_mut(a.as_slice_mut().unwrap().as_ptr() as * mut f32 , a.len());
                let mut bf32 = std::slice::from_raw_parts_mut(b.as_slice_mut().unwrap().as_ptr() as * mut f32 , b.len());
                let ires = lapacke::sggsvd3(Layout::RowMajor, jobu, jobv, jobq, 
                        //nb row of m , nb columns , nb row of n
                        a_nbrow.try_into().unwrap(), a_nbcol.try_into().unwrap(), b.dim().0.try_into().unwrap(),
                        &mut k, &mut l,
                        &mut af32, lda,
                        &mut bf32, ldb,
                        alpha_f32.as_mut_slice(),beta_f32.as_mut_slice(),
                        u_f32.as_slice_mut().unwrap(), ldu,
                        v_f32.as_slice_mut().unwrap(), ldv,
                        q_f32.as_mut_slice(), ldq,
                        iwork.as_mut_slice());
                if ires == 0 {
                    // but now we must  transform u,v, alpha and beta from f32 to F
                    u = ndarray::ArrayView::<F, Ix2>::from_shape_ptr(u_f32.dim(), u_f32.as_ptr() as *const F).into_owned();
                    v = ndarray::ArrayView::<F, Ix2>::from_shape_ptr(v_f32.dim(), v_f32.as_ptr() as *const F).into_owned();
                    alpha = ndarray::ArrayView::<F, Ix1>::from_shape_ptr((alpha_f32.len()),alpha_f32.as_ptr() as *const F).into_owned();
                    beta = ndarray::ArrayView::<F, Ix1>::from_shape_ptr((beta_f32.len()),beta_f32.as_ptr() as *const F).into_owned();
                    // TODO fill in gsvdres
                    gsvdres.init_from_lapack(u, v, k, l , alpha , beta);
                }
                else if ires == 1 {
                    return Err(anyhow!("lapack failed to converge"));
                }
                else if ires < 0 {
                    return Err(anyhow!("argument {} had an illegal value", -ires));
                }
                //
                ires
            }; // end of unsafe block
            // test ires
        }  // end case f32
        else if TypeId::of::<F>() == TypeId::of::<f64>() {
            let mut alpha_f64 = Vec::<f64>::with_capacity(a_nbcol);
            let mut beta_f64 = Vec::<f64>::with_capacity(a_nbcol);
            let mut u_f64= Array2::<f64>::zeros((a_nbrow, a_nbrow));
            let mut v_f64= Array2::<f64>::zeros((b_dim.0, b_dim.0));
            let mut q_f64 = Vec::<f64>::new(); 
            ires = unsafe {
                let mut af64 = std::slice::from_raw_parts_mut(a.as_slice_mut().unwrap().as_ptr() as * mut f64 , a.len());
                let mut bf64 = std::slice::from_raw_parts_mut(b.as_slice_mut().unwrap().as_ptr() as * mut f64 , b.len()); 
                let ires = lapacke::dggsvd3(Layout::RowMajor, jobu, jobv, jobq, 
                    //nb row of m , nb columns , nb row of n
                    a_nbrow.try_into().unwrap(), a_nbcol.try_into().unwrap(), b.dim().0.try_into().unwrap(),
                    &mut k, &mut l,
                    &mut af64, lda,
                    &mut bf64, ldb,
                    alpha_f64.as_mut_slice(),beta_f64.as_mut_slice(),
                    u_f64.as_slice_mut().unwrap(), ldu,
                    v_f64.as_slice_mut().unwrap(), ldv,
                    q_f64.as_mut_slice(), ldq,
                    iwork.as_mut_slice());
                // but now we must transform u,v, alpha and beta from f64 to F
                if ires == 0 {
                    u = ndarray::ArrayView::<F, Ix2>::from_shape_ptr(u_f64.dim(), u_f64.as_ptr() as *const F).into_owned();
                    v = ndarray::ArrayView::<F, Ix2>::from_shape_ptr(v_f64.dim(), v_f64.as_ptr() as *const F).into_owned();
                    alpha = ndarray::ArrayView::<F, Ix1>::from_shape_ptr((alpha_f64.len()),alpha_f64.as_ptr() as *const F).into_owned();
                    beta = ndarray::ArrayView::<F, Ix1>::from_shape_ptr((beta_f64.len()),beta_f64.as_ptr() as *const F).into_owned();
                    gsvdres.init_from_lapack(u, v, k, l , alpha , beta);
                }
                else if ires == 1 {
                    return Err(anyhow!("lapack failed to converge"));
                }
                else if ires < 0 {
                    return Err(anyhow!("argument {} had an illegal value", -ires));
                }                
                ires
            }           
        }  // end case f64
        else {
            log::error!("do_approx_gsvd only implemented for f32 just now!");
            panic!();
        }
        // Ok(())
        Err(anyhow!("not yet implemented"))
    }  // end of do_approx_gsvd

} // end of impl block for GSvdApprox




mod tests {

#[allow(unused)]
use super::*;

#[allow(unused)]
use sprs::{CsMat, TriMatBase};

#[allow(dead_code)]
fn log_init_test() {
    let _ = env_logger::builder().is_test(true).try_init();
}  


#[test]
// small example from https://fr.mathworks.com/help/matlab/ref/gsvd.html
// with more rows than columns. run in precision mode

fn test_lapack() {
    log_init_test();
    let mat_a = [ [1., 6., 11.],[2., 7., 12.] , [3., 8., 13.], [4., 9., 14.], [5., 10., 15.] ];
    let mat_b = [ [8., 1., 6.],[3., 5., 7.] , [4., 9., 2.]];
    // convert in csr modde !!

}


fn test_gsvd_full_precision_1() {
    log_init_test();
    //
    let mat_a = [ [1., 6., 11.],[2., 7., 12.] , [3., 8., 13.], [4., 9., 14.], [5., 10., 15.] ];
    let mat_b = [ [8., 1., 6.],[3., 5., 7.] , [4., 9., 2.]];
    // convert in csr modde !!

} // end of test_gsv_full_1

// The smae test as test_gsvd_full_1 but with matrix described in csr mode, run in precision mode
fn test_gsvd_csr_precision_1() {
    log_init_test();
    //
    let mat_a = [ [1., 6., 11.],[2., 7., 12.] , [3., 8., 13.], [4., 9., 14.], [5., 10., 15.] ];
    let mat_b = [ [8., 1., 6.],[3., 5., 7.] , [4., 9., 2.]];
    // convert in csr modde !!

}

// we h ve fumm matrix we can test in rank mode
fn test_gsvd_full_rank_1() {
    log_init_test();
    //
    let mat_a = [ [1., 6., 11.],[2., 7., 12.] , [3., 8., 13.], [4., 9., 14.], [5., 10., 15.] ];

    let mat_b = [ [8., 1., 6.],[3., 5., 7.] , [4., 9., 2.]];

} // end of test_gsvd_full_rank_1

} // end of mod tests    

