//! This file implements interface to lapack gsvd
//! 
//! 


use log::Level::Debug;
use log::{log_enabled};
use anyhow::{anyhow};


use num_traits::float::*;

use ndarray_linalg::{Scalar, Lapack};
use std::any::TypeId;
use ndarray::{s,Array1, Array2, ArrayView2, ArrayBase, ViewRepr, Dim, Ix1, Ix2};

use lapacke::{Layout};





#[derive(Copy, Clone, Debug)]
/// This structure describes optional parameters used to specify the Gsvd approximation to do by GSvdApprox
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



pub struct GSvd<'a, F: Scalar> {
    /// first matrix we want to approximate range of
    a : &'a mut Array2<F>,
    /// second matrix
    b : &'a mut Array2<F>,
    /// optional parameters
    opt_params : Option<GSvdOptParams>,
}   // end of struct GsvdApprox



#[cfg_attr(doc, katexit::katexit)]
/// For a Standard Gvsd problem described by the pair of matrix mat_1 (m,n) and mat_2 (p,n)
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
pub struct GSvdResult<F: Float + Scalar> {
    /// left eigenvectors for first matrix. U
    pub(crate)  v1 : Option<Array2<F>>,
    /// left eigenvectors. (m,r) matrix where r is rank asked for and m the number of data.
    pub(crate)  v2 : Option<Array2<F>>,
    /// first (diagonal matrix) eigenvalues
    pub(crate)  s1 : Option<Array1<F>>,
    /// second (diagonal matrix) eigenvalues
    pub(crate)  s2 : Option<Array1<F>>,
    /// common right term of mat1 and mat2 factorization if asked for
    pub(crate) _commonx : Option<Array2<F>>
} // end of struct SvdResult<F> 


impl <F> GSvdResult<F>  where  F : Float + Lapack + Scalar + ndarray::ScalarOperand + sprs::MulAcc  {

    pub(crate) fn new() -> Self {
        GSvdResult{v1 :None, v2 : None, s1 : None, s2 : None, _commonx :None}
    }

    // reconstruct result from the out parameters of lapack. For us u and v are always asked for
    // (m,n) is dimension of A. p is number of rows of B. k and l oare lapack output  
    pub(crate) fn init_from_lapack(&mut self, m : i64, n : i64, p : i64, u : Array2<F>, v : Array2<F>, k : i64 ,l : i64 , 
                alpha : Array1<F>, beta : Array1<F>, _permuta : Array1<i32>) {
        self.v1 = Some(u);
        self.v2 = Some(v);
        // now we must decode depending upon k and l values, we use the lapack doc at :
        // http://www.netlib.org/lapack/explore-html/d1/d7e/group__double_g_esing_gab6c743f531c1b87922eb811cbc3ef645.html
        //
        log::debug!("m : {}, n : {}, p : {}, k : {}, l : {} ", m, n, p, k, l);
        assert!(m >= 0);
        assert!(l >= 0);
        assert!(k >= 0);
        //
        let s1_v : ArrayBase<ViewRepr<&F>, Dim<[usize;1]>>;
        let s2_v : ArrayBase<ViewRepr<&F>, Dim<[usize;1]>>;
        // on 0..k  alpha = 1. beta = 0.
        if m-k-l >= 0 {
            log::debug!("m-k-l >= 0");
            // s1 is alpha[k .. k+l-1] and   s2 is beta[k .. k+l-1], 
            assert!(l > 0);
            assert!(k >= 0);
            s1_v = alpha.slice(s![k as usize ..(k+l) as usize]);
            s2_v = beta.slice(s![k as usize ..(k+l) as usize]);
        }
        else {
            log::debug!("m-k-l < 0");
            // s1 is alpha[k..m]  and s2 is beta[k..m], alpha[m..k+l] == 0 and beta[m..k+l] == 1 and beyond k+l  alpha = beta == 0
            assert!(k >= 0);           
            assert!(m >= k);
            s1_v = alpha.slice(s![k as usize..(m as usize)]);
            s2_v = beta.slice(s![k as usize..(m as usize)]);
        }
        // a dump if log Debug enabled we dump alpha, beta and C and S in the middle range of alpha and beta
        if log_enabled!(Debug) {
            for i in 0..k as usize {
                log::debug!(" i {}, alpha[i] {},  beta[i] {}", i, alpha[i], beta[i]);
            }
            for i in 0..s1_v.len() {
                log::debug!(" i {}, C[i] {},  S[i] {}", i, s1_v[i], s2_v[i]);
            }
            for i in (k+l).min(m) as usize..n as usize {
                log::debug!(" i {}, alpha[i] {},  beta[i] {}", i, alpha[i], beta[i]);
            }
        }
        // some checks
        let check : Vec<F> = s1_v.iter().zip(s2_v.iter()).map(| x |  *x.0 * *x.0 + *x.1 * *x.1).collect();
        for v in check {
            let epsil = (1. - v.to_f64().unwrap()).abs();
            log::debug!(" epsil = {}", epsil);
            assert!(epsil < 1.0E-5 );
        }
        // we clone
        self.s1 = Some(s1_v.to_owned());
        self.s2 = Some(s2_v.to_owned());
        let k_1 = k as usize;
        let s = m.min(k+l) as usize;
        // sorting
        /* 
        let mut alpha_sorted = alpha.clone();
        for i in k_1..s {
            log::debug!("i {} , permuta[i] {}", i , permuta[i]);
            alpha_sorted.swap(i, permuta[i] as usize);
        }  */
        // It seems alpha is sorted. we check
        for i in k_1+1..s {
            if alpha[i] > alpha[i-1] {
                log::error!("alpha non decreasing at i : {}  {}  {}", i, alpha[i], alpha[i-1]);
                panic!("non sorted alpha");
            }
        }
        // possibly commonx (or Q in Lapack docs) but here we do not keep it
    }  // end of GSvdResult::init_from_lapack



    // debug utility for tests!
    #[allow(unused)]
    pub(crate) fn dump_u(&self) {
        if self.v1.is_some() {
            let u = self.v1.as_ref().unwrap();
            log::debug!("\n dumping U");
            dump::<F>(&u.view());
        }
    }  // end of dump_u

    // we check that u and v are orthogonal
    #[allow(unused)]
    pub(crate) fn check_uv_orthogonal(&self) -> Result<(),()> {
        if self.v1.is_some() {
            let u = self.v1.as_ref().unwrap();
            let res = check_orthogonality::<F>(u);
            if res.is_err() {
                return res;
            }
        }
        if self.v2.is_some() {
            let v = self.v2.as_ref().unwrap();
            if log_enabled!(Debug) {
                println!("\n\n dumping v");
                dump::<F>(&v.view());
            }
            let res = check_orthogonality::<F>(v);
            if res.is_err() {
                return res;
            }
        }
        //
        Ok(())
    }  // end of check_u_orthogonal

} // end of impl block for GSvdResult



pub(crate) fn dump<F>(a : &ArrayView2<F>) where F : Float + Lapack + Scalar {
    for i in 0..a.dim().0 {
        println!();
        for j in 0..a.dim().1 {
            print!("{:.3e} ", a[[i,j]]);
        }
    }
} // end of dump



pub(crate) fn check_orthogonality<F>(u: &Array2<F>) -> Result<(),()> 
             where F : Float + Lapack + Scalar {
    //
    let epsil = 1.0E-5;
    //
    let id : Array2<F> = u.dot(&u.t()); 
    if log_enabled!(Debug) {
        println!("\n\n\n dump a*t(a)");
        dump::<F>(&id.view());
    }
    let n = id.dim().0;
    for i in 0..n {
        if (1. - id[[i,i]].to_f64().unwrap()).abs() > epsil {
            log::error!("check_orthogonality failed at ({},{})", i,i);
            return Err(());
        }
        for j in 0..i {
            if (id[[i,j]].to_f64().unwrap()).abs() > epsil {
                log::error!("check_orthogonality failed at ({},{})", i,j);
                return Err(());
            }                    
        }
    }       
    //
    Ok(())
}  // end check orthogonality



//=========================================================================



impl  <'a, F> GSvd<'a, F>  
    where  F : Float + Lapack + Scalar  + ndarray::ScalarOperand + sprs::MulAcc {
    /// We impose the RangePrecision mode for now.
    pub fn new(a : &'a mut Array2<F>, b : &'a mut Array2<F>) -> Self {
        // check for dimensions constraints
        if a.dim().1 != b.dim().1 {
            log::error!("The two matrices for gsvd must have the same number of columns");
            println!("The two matrices for gsvd must have the same number of columns");
            panic!("Error constructiing Gsvd problem");
        }
        return GSvd{a, b, opt_params:None};
    } // end of new

    /// return optional paramertes if any
    pub fn get_parameters(&self) -> &Option<GSvdOptParams> {
        &self.opt_params
    } // end of set_parameters



    // We have to :
    //   - do a range approximation of the 2 matrices in problem definition
    //   - do a (full) gsvd of the 2 reduced matrices 
    //   - lapck rust interface requires we pass matrix as slices so they must be in row order!
    //     but for our application we must pass transposed version of Mg and Ml as we must compute inverse(Mg) * Ml
    //     with a = Mg and b = Ml. So it seems we cannot avoid copying when construction the GSvdApprox

    /// 
    pub fn do_gsvd(&mut self) -> Result<GSvdResult<F>, anyhow::Error> {

        // now we must do the standard generalized svd (with Lapack ggsvd3) for m and reduced_n
        // We are at step iv) of algo 2.4 of Wei and al.
        // See rust doc https://docs.rs/lapacke/latest/lapacke/fn.dggsvd3.html and
        // fortran https://www.netlib.org/lapack/lug/node36.html#1815 but new function is (s|d)ggsvd3
        //
        // Lapack definition of GSVD is in the following link:
        // http://www.netlib.org/lapack/explore-html/d1/d7e/group__double_g_esing_gab6c743f531c1b87922eb811cbc3ef645.html
        //
        //  Lapack GSVD(A,B) for A=(m,n) and B=(p,n) 
        //  gives U**T*A*Q = D1*( 0 R ),    V**T*B*Q = D2*( 0 R )   with  U , V and Q orthogonals
        //
        let (a_nbrow, a_nbcol) = self.a.dim();
        let jobu = b'U';
        let jobv = b'V';
        let jobq = b'N';        // Q is large we do not need it, we do not compute it
        assert_eq!(a_nbcol, self.b.dim().1); // check m and n have the same number of columns.
        let mut k : i32 = 0;
        let mut l : i32 = 0;
        // for lda  see lapacke interface  : http://www.netlib.org/lapack/lapacke.html#_array_arguments
        // Caution our matrix are C (row) ordered so lda is nbcol. but we want to send the transpose (!) so lda is a_nbrow
        let lda : i32 = a_nbcol as i32;
        let b_dim = self.b.dim();
        // caution our matrix are C (row) ordered so lda is nbcol. but we want to send the transpose (!) so lda is a_nbrow
        let ldb : i32 = b_dim.1 as i32;
        let _ires: i32;
        let ldu = a_nbrow as i32;  // ldu must be greater equal nb rows of A.  as U = (a_nbrow, a_nbrow)
        let ldv = b_dim.0 as i32;  // ldv is b_nbcol as V = (b_nbcol, b_nbcol)
        //
        let ldq : i32 = a_nbcol as i32;  // as we do not ask for Q but test test_lapack_array showed we cannot set to 1!
        let mut iwork = Array1::<i32>::zeros(a_nbcol);
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
            _ires = unsafe {
                // we must cast a and b to f32 slices!! unsafe but we know our types with TypeId
                let mut af32 = std::slice::from_raw_parts_mut(self.a.as_slice_mut().unwrap().as_ptr() as * mut f32 , self.a.len());
                let mut bf32 = std::slice::from_raw_parts_mut(self.b.as_slice_mut().unwrap().as_ptr() as * mut f32 , self.b.len());
                let ires = lapacke::sggsvd3(Layout::RowMajor, jobu, jobv, jobq, 
                        //nb row of m , nb columns , nb row of n
                        a_nbrow.try_into().unwrap(), a_nbcol.try_into().unwrap(), self.b.dim().0.try_into().unwrap(),
                        &mut k, &mut l,
                        &mut af32, lda,
                        &mut bf32, ldb,
                        alpha_f32.as_mut_slice(),beta_f32.as_mut_slice(),
                        u_f32.as_slice_mut().unwrap(), ldu,
                        v_f32.as_slice_mut().unwrap(), ldv,
                        q_f32.as_mut_slice(), ldq,
                        iwork.as_slice_mut().unwrap());
                if ires == 0 {
                    // but now we must  transform u,v, alpha and beta from f32 to F
                    u = ndarray::ArrayView::<F, Ix2>::from_shape_ptr(u_f32.dim(), u_f32.as_ptr() as *const F).into_owned();
                    v = ndarray::ArrayView::<F, Ix2>::from_shape_ptr(v_f32.dim(), v_f32.as_ptr() as *const F).into_owned();
                    alpha = ndarray::ArrayView::<F, Ix1>::from_shape_ptr(alpha_f32.len(),alpha_f32.as_ptr() as *const F).into_owned();
                    beta = ndarray::ArrayView::<F, Ix1>::from_shape_ptr(beta_f32.len(),beta_f32.as_ptr() as *const F).into_owned();
                    // convert usize to i64 as matrix sizes surely permits that
                    gsvdres.init_from_lapack(a_nbrow.try_into().unwrap(), a_nbcol.try_into().unwrap() , b_dim.0.try_into().unwrap(), 
                                u, v, k as i64, l as i64 , alpha , beta, iwork);
                }
                else if ires == 1 {
                    return Err(anyhow!("lapack for f64 failed to converge"));
                }
                else if ires < 0 {
                    return Err(anyhow!("argument {} had an illegal value", -ires));
                }
                //
                ires
            }; // end of unsafe block
        }  // end case f32
        else if TypeId::of::<F>() == TypeId::of::<f64>() {
            let mut alpha_f64 = Vec::<f64>::with_capacity(a_nbcol);
            let mut beta_f64 = Vec::<f64>::with_capacity(a_nbcol);
            let mut u_f64= Array2::<f64>::zeros((a_nbrow, a_nbrow));
            let mut v_f64= Array2::<f64>::zeros((b_dim.0, b_dim.0));
            let mut q_f64 = Vec::<f64>::new(); 
            _ires = unsafe {
                let mut af64 = std::slice::from_raw_parts_mut(self.a.as_slice_mut().unwrap().as_ptr() as * mut f64 , self.a.len());
                let mut bf64 = std::slice::from_raw_parts_mut(self.b.as_slice_mut().unwrap().as_ptr() as * mut f64 , self.b.len()); 
                let ires = lapacke::dggsvd3(Layout::RowMajor, jobu, jobv, jobq, 
                    //nb row of m , nb columns , nb row of n
                    a_nbrow.try_into().unwrap(), a_nbcol.try_into().unwrap(), self.b.dim().0.try_into().unwrap(),
                    &mut k, &mut l,
                    &mut af64, lda,
                    &mut bf64, ldb,
                    alpha_f64.as_mut_slice(),beta_f64.as_mut_slice(),
                    u_f64.as_slice_mut().unwrap(), ldu,
                    v_f64.as_slice_mut().unwrap(), ldv,
                    q_f64.as_mut_slice(), ldq,
                    iwork.as_slice_mut().unwrap());
                // but now we must transform u,v, alpha and beta from f64 to F
                if ires == 0 {
                    u = ndarray::ArrayView::<F, Ix2>::from_shape_ptr(u_f64.dim(), u_f64.as_ptr() as *const F).into_owned();
                    v = ndarray::ArrayView::<F, Ix2>::from_shape_ptr(v_f64.dim(), v_f64.as_ptr() as *const F).into_owned();
                    alpha = ndarray::ArrayView::<F, Ix1>::from_shape_ptr(alpha_f64.len(),alpha_f64.as_ptr() as *const F).into_owned();
                    beta = ndarray::ArrayView::<F, Ix1>::from_shape_ptr(beta_f64.len(),beta_f64.as_ptr() as *const F).into_owned();
                    gsvdres.init_from_lapack(a_nbrow.try_into().unwrap(), a_nbcol.try_into().unwrap() , b_dim.0.try_into().unwrap(), 
                            u, v, k as i64, l as i64 , alpha , beta, iwork);
                }
                else if ires == 1 {
                    return Err(anyhow!("lapack for f32 failed to converge"));
                }
                else if ires < 0 {
                    return Err(anyhow!("argument {} had an illegal value", -ires));
                }                
                ires
            }  // end unsafe         
        }  // end case f64
        else {
            log::error!("do_approx_gsvd only implemented for f32 and f64");
            panic!();
        }
        Ok(gsvdres)
    }  // end of do_approx_gsvd

} // end of impl block for Gsvd


//===============================================================================

// Run with for example : RUST_LOG=DEBUG cargo test test_lapack_gsvd_array_2 -- --nocapture
mod tests {

use super::*;

#[allow(unused_imports)]  // rust analyzer pb we need it!
use ndarray::{array};

#[allow(unused)]
fn log_init_test() {
    let _ = env_logger::builder().is_test(true).try_init();
}  


// with more rows than columns.run in precision mode

#[allow(unused)]
fn small_lapack_gsvd(a: &mut Array2<f64>, b : &mut Array2<f64>) -> GSvdResult::<f64> {
    //
    let (a_nbrow, a_nbcol) = a.dim();
    log::debug!("a dims : ({}, {})", a_nbrow, a_nbcol);
    let jobu = 'U' as u8;   // we compute U
    let jobv = b'V';   // we compute V
    let jobq = 'N' as u8;   // Q is large we do not need it, we do not compute it
    assert_eq!(a_nbcol, b.dim().1); // check m and n have the same number of columns.
    let mut k : i32 = 0;
    let mut l : i32 = 0;
    let lda : i32 = a_nbcol as i32;  // our matrix are row ordered and see https://www.netlib.org/lapack/lapacke.html
    let b_dim = b.dim();
    log::debug!("b dims : ({}, {})", b_dim.0, b_dim.1);
    let ldb : i32 = b_dim.1 as i32;     // our matrix are row ordered!
    let mut alpha_f64 = Array1::<f64>::zeros(a_nbcol);
    let mut beta_f64 = Array1::<f64>::zeros(a_nbcol);
    let mut u_f64= Array2::<f64>::zeros((a_nbrow, a_nbrow));
    let mut v_f64= Array2::<f64>::zeros((b_dim.0, b_dim.0));
    let mut q_f64 = Vec::<f64>::new(); 
    let ldu = a_nbrow as i32;  // as we compute U , ldu must be greater than nb rows of A lapack doc
    let ldv = b_dim.0 as i32;
    // The following deviates from doc http://www.netlib.org/lapack/explore-html/d1/d7e/group__double_g_esing_gab6c743f531c1b87922eb811cbc3ef645.html
    let ldq = a_nbcol as i32;    // we do not ask for Q but ldq must be >= a_nbcol (error msg from LAPACKE_dggsvd3_work)
    let mut iwork = Array1::<i32>::zeros(a_nbcol);
    // lda parameter 11, ldv parameter 19  in dggsvd3 and 
    let ires = unsafe {
        let a_slice = std::slice::from_raw_parts_mut(a.as_slice_mut().unwrap().as_ptr() as *mut f64 , a.len());
        let b_slice = std::slice::from_raw_parts_mut(b.as_slice_mut().unwrap().as_ptr() as *mut f64 , b.len()); 
        lapacke::dggsvd3(Layout::RowMajor, jobu, jobv, jobq, 
                //nb row of m , nb columns , nb row of n
                a_nbrow.try_into().unwrap(), a_nbcol.try_into().unwrap(), b.dim().0.try_into().unwrap(),
                &mut k, &mut l,
                a_slice, lda, b_slice, ldb,
                alpha_f64.as_slice_mut().unwrap(),beta_f64.as_slice_mut().unwrap(),
                u_f64.as_slice_mut().unwrap(), ldu,
                v_f64.as_slice_mut().unwrap(), ldv,
                q_f64.as_mut_slice(), ldq,
                iwork.as_slice_mut().unwrap()
        )
    };
    // 
    if ires != 0 {
        println!("ggsvd3 returned {}", ires);
        log::error!("dggsvd3 returned {}", ires);
        assert!(1==0);
    }
    log::debug!("dggsvd3 passed");
    // allocate result
    let mut gsvdres = GSvdResult::<f64>::new();
    gsvdres.init_from_lapack(a_nbrow.try_into().unwrap(), a_nbcol.try_into().unwrap(), b_dim.0.try_into().unwrap(), 
            u_f64, v_f64, k.into(), l.into(), alpha_f64, beta_f64, iwork);
    //
    gsvdres
}   // end of small_lapack_gsvd






#[test]
// a test to check rust lapack interface more rows than columns
// small example from https://fr.mathworks.com/help/matlab/ref/gsvd.html
fn test_lapack_gsvd_array_1() {
    log_init_test();
    //
    let mut a = array![ [1., 6., 11.], [2., 7., 12.] , [3., 8., 13.], [4., 9., 14.], [5., 10., 15.] ];
    let mut b = array![ [8., 1., 6.],[3., 5., 7.] , [4., 9., 2.]];
    let gsvdres = small_lapack_gsvd(&mut a, &mut b);
    // dump results
    gsvdres.dump_u();
    let res = gsvdres.check_uv_orthogonal();
    assert!(res.is_ok());
} // end of test_lapack_gsvd_array



// test with more columns than rows
// taken from https://rdrr.io/cran/geigen/man/gsvd.html
#[test]
fn test_lapack_gsvd_array_2() {
    log_init_test();
    //
    let mut a = array![ [ 1. , 2. , 3. , 3.,  2. , 1.] , [ 4. , 5. , 6. , 7. , 8., 8.]   ];
    let mut b = array![ [1., 2., 3., 4., 5., 6.] , 
                                                [ 7. , 8., 9., 10., 11., 12.] , 
                                                [ 13. , 14., 15., 16., 17., 18.]   ];
    let gsvdres = small_lapack_gsvd(&mut a, &mut b);
    // dump results
    gsvdres.dump_u();
    let res = gsvdres.check_uv_orthogonal();
    assert!(res.is_ok());
} // end of test_lapack_gsvd_array_2



}  // end of mod tests


