use syn::{Expr, LitBool, LitStr};

pub(crate) fn get_lit_str<'a>(attr_name: &str, value: &'a Expr) -> syn::Result<&'a LitStr> {
    match value {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Str(lit) => Ok(lit),
            _ => Err(syn::Error::new_spanned(
                value,
                format!("expected `{attr_name}` to be a string literal"),
            )),
        },
        _ => Err(syn::Error::new_spanned(
            value,
            format!("expected `{attr_name}` to be a string literal"),
        )),
    }
}

pub(crate) fn get_lit_bool(attr_name: &str, value: &Expr) -> syn::Result<bool> {
    match value {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Bool(LitBool { value, .. }) => Ok(*value),
            _ => Err(syn::Error::new_spanned(
                value,
                format!("expected `{attr_name}` to be a boolean literal"),
            )),
        },
        _ => Err(syn::Error::new_spanned(
            value,
            format!("expected `{attr_name}` to be a boolean literal"),
        )),
    }
}
