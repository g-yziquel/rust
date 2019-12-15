use crate::consts::{
    constant,
    Constant::{F32, F64},
};
use crate::utils::*;
use if_chain::if_chain;
use rustc::declare_lint_pass;
use rustc::hir::*;
use rustc::lint::{LateContext, LateLintPass, LintArray, LintPass};
use rustc_errors::Applicability;
use rustc_session::declare_tool_lint;
use std::f32::consts as f32_consts;
use std::f64::consts as f64_consts;

declare_clippy_lint! {
    /// **What it does:** Looks for floating-point expressions that
    /// can be expressed using built-in methods to improve accuracy,
    /// performance and/or succinctness.
    ///
    /// **Why is this bad?** Negatively affects accuracy, performance
    /// and/or readability.
    ///
    /// **Known problems:** None
    ///
    /// **Example:**
    ///
    /// ```rust
    /// use std::f32::consts::E;
    ///
    /// let a = 3f32;
    /// let _ = (2f32).powf(a);
    /// let _ = E.powf(a);
    /// let _ = a.powf(1.0 / 2.0);
    /// let _ = a.powf(1.0 / 3.0);
    /// let _ = a.log(2.0);
    /// let _ = a.log(10.0);
    /// let _ = a.log(E);
    /// let _ = (1.0 + a).ln();
    /// let _ = a.exp() - 1.0;
    /// ```
    ///
    /// is better expressed as
    ///
    /// ```rust
    /// use std::f32::consts::E;
    ///
    /// let a = 3f32;
    /// let _ = a.exp2();
    /// let _ = a.exp();
    /// let _ = a.sqrt();
    /// let _ = a.cbrt();
    /// let _ = a.log2();
    /// let _ = a.log10();
    /// let _ = a.ln();
    /// let _ = a.ln_1p();
    /// let _ = a.exp_m1();
    /// ```
    pub FLOATING_POINT_IMPROVEMENTS,
    nursery,
    "looks for improvements to floating-point expressions"
}

declare_lint_pass!(FloatingPointArithmetic => [FLOATING_POINT_IMPROVEMENTS]);

fn check_log_base(cx: &LateContext<'_, '_>, expr: &Expr, args: &HirVec<Expr>) {
    let arg = sugg::Sugg::hir(cx, &args[0], "..").maybe_par();

    if let Some((value, _)) = constant(cx, cx.tables, &args[1]) {
        let method;

        if F32(2.0) == value || F64(2.0) == value {
            method = "log2";
        } else if F32(10.0) == value || F64(10.0) == value {
            method = "log10";
        } else if F32(f32_consts::E) == value || F64(f64_consts::E) == value {
            method = "ln";
        } else {
            return;
        }

        span_lint_and_sugg(
            cx,
            FLOATING_POINT_IMPROVEMENTS,
            expr.span,
            "logarithm for bases 2, 10 and e can be computed more accurately",
            "consider using",
            format!("{}.{}()", arg, method),
            Applicability::MachineApplicable,
        );
    }
}

// TODO: Lint expressions of the form `(x + 1).ln()` and `(x + y).ln()`
// where y > 1 and suggest usage of `(x + (y - 1)).ln_1p()` instead
fn check_ln1p(cx: &LateContext<'_, '_>, expr: &Expr, args: &HirVec<Expr>) {
    if_chain! {
        if let ExprKind::Binary(op, ref lhs, ref rhs) = &args[0].kind;
        if op.node == BinOpKind::Add;
        if let Some((value, _)) = constant(cx, cx.tables, lhs);
        if F32(1.0) == value || F64(1.0) == value;
        then {
            let arg = sugg::Sugg::hir(cx, rhs, "..").maybe_par();

            span_lint_and_sugg(
                cx,
                FLOATING_POINT_IMPROVEMENTS,
                expr.span,
                "ln(1 + x) can be computed more accurately",
                "consider using",
                format!("{}.ln_1p()", arg),
                Applicability::MachineApplicable,
            );
        }
    }
}

fn check_powf(cx: &LateContext<'_, '_>, expr: &Expr, args: &HirVec<Expr>) {
    // Check receiver
    if let Some((value, _)) = constant(cx, cx.tables, &args[0]) {
        let method;

        if F32(f32_consts::E) == value || F64(f64_consts::E) == value {
            method = "exp";
        } else if F32(2.0) == value || F64(2.0) == value {
            method = "exp2";
        } else {
            return;
        }

        span_lint_and_sugg(
            cx,
            FLOATING_POINT_IMPROVEMENTS,
            expr.span,
            "exponent for bases 2 and e can be computed more accurately",
            "consider using",
            format!("{}.{}()", sugg::Sugg::hir(cx, &args[1], "..").maybe_par(), method),
            Applicability::MachineApplicable,
        );
    }

    // Check argument
    if let Some((value, _)) = constant(cx, cx.tables, &args[1]) {
        let help;
        let method;

        if F32(1.0 / 2.0) == value || F64(1.0 / 2.0) == value {
            help = "square-root of a number can be computed more efficiently and accurately";
            method = "sqrt";
        } else if F32(1.0 / 3.0) == value || F64(1.0 / 3.0) == value {
            help = "cube-root of a number can be computed more accurately";
            method = "cbrt";
        } else {
            return;
        }

        span_lint_and_sugg(
            cx,
            FLOATING_POINT_IMPROVEMENTS,
            expr.span,
            help,
            "consider using",
            format!("{}.{}()", sugg::Sugg::hir(cx, &args[0], ".."), method),
            Applicability::MachineApplicable,
        );
    }
}

// TODO: Lint expressions of the form `x.exp() - y` where y > 1
// and suggest usage of `x.exp_m1() - (y - 1)` instead
fn check_expm1(cx: &LateContext<'_, '_>, expr: &Expr) {
    if_chain! {
        if let ExprKind::Binary(op, ref lhs, ref rhs) = expr.kind;
        if op.node == BinOpKind::Sub;
        if cx.tables.expr_ty(lhs).is_floating_point();
        if let Some((value, _)) = constant(cx, cx.tables, rhs);
        if F32(1.0) == value || F64(1.0) == value;
        if let ExprKind::MethodCall(ref path, _, ref method_args) = lhs.kind;
        if path.ident.name.as_str() == "exp";
        then {
            span_lint_and_sugg(
                cx,
                FLOATING_POINT_IMPROVEMENTS,
                expr.span,
                "(e.pow(x) - 1) can be computed more accurately",
                "consider using",
                format!(
                    "{}.exp_m1()",
                    sugg::Sugg::hir(cx, &method_args[0], "..")
                ),
                Applicability::MachineApplicable,
            );
        }
    }
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for FloatingPointArithmetic {
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr) {
        if let ExprKind::MethodCall(ref path, _, args) = &expr.kind {
            let recv_ty = cx.tables.expr_ty(&args[0]);

            if recv_ty.is_floating_point() {
                match &*path.ident.name.as_str() {
                    "ln" => check_ln1p(cx, expr, args),
                    "log" => check_log_base(cx, expr, args),
                    "powf" => check_powf(cx, expr, args),
                    _ => {},
                }
            }
        } else {
            check_expm1(cx, expr);
        }
    }
}
