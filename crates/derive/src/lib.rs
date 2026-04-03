use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Lit, parse_macro_input};

/// Derive macro for generating a debug pane from a `Resource` struct.
///
/// No `Reflect` needed. Generates typed `read_field`/`write_field` accessors
/// and bidirectional sync between the resource and the UI.
///
/// # Struct-level attributes
///
/// - `#[pane(title = "...")]` — pane title (defaults to struct name)
/// - `#[pane(position = "...")]` — `"top-left"`, `"top-right"`, `"bottom-left"`,
///   `"bottom-right"`, or `"x, y"` for absolute positioning
///
/// # Field-level attributes
///
/// - `#[pane(skip)]` — exclude from pane
/// - `#[pane(slider, min = 0.0, max = 10.0, step = 0.1)]` — slider control
/// - `#[pane(number)]` — number input
/// - `#[pane(color)]` — color picker
/// - `#[pane(monitor)]` — read-only display (game writes, UI shows)
/// - `#[pane(select(options = ["A", "B", "C"]))]` — dropdown
/// - `#[pane(custom = "vector2")]` — custom plugin control
/// - `#[pane(default = V)]` — field default (generates `Default` impl)
/// - `#[pane(label = "Display Name")]` — override display label
/// - `#[pane(folder = "Group")]` — place in collapsible folder
/// - `#[pane(tab = "Page")]` — place in tab page
/// - `#[pane(tooltip = "Help text")]` — hover tooltip
/// - `#[pane(icon = "svg_string")]` — SVG icon in label
/// - `#[pane(order = N)]` — display order
/// - `#[pane(min = N, max = N, step = N)]` — numeric constraints
///
/// # Auto-detection
///
/// Fields without explicit control attributes are auto-detected:
/// `bool` → toggle, `String` → text, `Color` → color picker,
/// `f32`/`f64`/integers → number input.
#[proc_macro_derive(Pane, attributes(pane))]
pub fn derive_pane(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // Parse struct-level #[pane(title = "...", position = "x, y")] attributes
    let mut title = struct_name.to_string();
    let mut position: Option<String> = None;
    for attr in &input.attrs {
        if !attr.path().is_ident("pane") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("title") {
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    title = s.value();
                }
            } else if meta.path.is_ident("position") {
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    position = Some(s.value());
                }
            }
            Ok(())
        });
    }

    // Parse fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "Pane derive only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "Pane derive only supports structs")
                .to_compile_error()
                .into();
        }
    };

    let mut field_descriptors = Vec::new();
    let mut read_arms = Vec::new();
    let mut write_arms = Vec::new();
    let mut default_fields = Vec::new();
    let mut has_any_pane_default = false;

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        let mut attrs = FieldAttrs::default();
        for attr in &field.attrs {
            if attr.path().is_ident("pane") {
                parse_field_attrs(attr, &mut attrs);
            }
        }

        // Collect default value expression for all fields (including skipped)
        if let Some(ref lit) = attrs.default_value {
            has_any_pane_default = true;
            let default_expr = lit_to_default_expr(lit);
            default_fields.push(quote! { #field_name: #default_expr });
        } else {
            default_fields.push(quote! { #field_name: Default::default() });
        }

        if attrs.skip {
            continue;
        }

        let label = attrs
            .label
            .clone()
            .unwrap_or_else(|| humanize_field_name(&field_name.to_string()));

        let field_name_str = field_name.to_string();
        let label_str = label;
        let folder_expr = opt_string(&attrs.folder);
        let tab_expr = opt_string(&attrs.tab);
        let tooltip_expr = opt_string(&attrs.tooltip);
        let icon_expr = opt_string(&attrs.icon);
        let order = attrs.order;

        let control_kind = determine_control_kind(&attrs, field_type);

        // Generate read_field / write_field match arms based on control kind
        let type_str = type_to_string(field_type);
        let (read_arm, write_arm) =
            field_rw_arms(&field_name_str, field_name, &type_str, &control_kind);
        read_arms.push(read_arm);
        write_arms.push(write_arm);

        let kind_expr = match &control_kind {
            ControlKind::Slider { min, max, step } => {
                quote! {
                    saddle_pane::binding::FieldControlKind::Slider {
                        min: #min, max: #max, step: #step,
                    }
                }
            }
            ControlKind::Number { min, max, step } => {
                let min_expr = opt_f64(min);
                let max_expr = opt_f64(max);
                quote! {
                    saddle_pane::binding::FieldControlKind::Number {
                        min: #min_expr, max: #max_expr, step: #step,
                    }
                }
            }
            ControlKind::Toggle => quote! { saddle_pane::binding::FieldControlKind::Toggle },
            ControlKind::Text => quote! { saddle_pane::binding::FieldControlKind::Text },
            ControlKind::Color => quote! { saddle_pane::binding::FieldControlKind::Color },
            ControlKind::Select { options } => {
                quote! {
                    saddle_pane::binding::FieldControlKind::Select {
                        options: vec![#(#options.to_string()),*],
                    }
                }
            }
            ControlKind::Monitor => quote! { saddle_pane::binding::FieldControlKind::Monitor },
            ControlKind::Custom { control_id, config } => {
                let keys: Vec<&String> = config.iter().map(|(k, _)| k).collect();
                let vals: Vec<&f64> = config.iter().map(|(_, v)| v).collect();
                quote! {
                    saddle_pane::binding::FieldControlKind::Custom {
                        control_id: #control_id.to_string(),
                        config: vec![#((#keys.to_string(), #vals)),*],
                    }
                }
            }
        };

        field_descriptors.push(quote! {
            saddle_pane::binding::FieldDescriptor {
                field_name: #field_name_str.to_string(),
                label: #label_str.to_string(),
                folder: #folder_expr,
                tab: #tab_expr,
                tooltip: #tooltip_expr,
                icon: #icon_expr,
                order: #order,
                kind: #kind_expr,
            }
        });
    }

    let title_str = title;

    let position_impl = match position.as_deref() {
        Some("top-left") | Some("TopLeft") => quote! {
            fn pane_position() -> Option<saddle_pane::builder::PanePosition> {
                Some(saddle_pane::builder::PanePosition::TopLeft)
            }
        },
        Some("top-right") | Some("TopRight") => quote! {
            fn pane_position() -> Option<saddle_pane::builder::PanePosition> {
                Some(saddle_pane::builder::PanePosition::TopRight)
            }
        },
        Some("bottom-left") | Some("BottomLeft") => quote! {
            fn pane_position() -> Option<saddle_pane::builder::PanePosition> {
                Some(saddle_pane::builder::PanePosition::BottomLeft)
            }
        },
        Some("bottom-right") | Some("BottomRight") => quote! {
            fn pane_position() -> Option<saddle_pane::builder::PanePosition> {
                Some(saddle_pane::builder::PanePosition::BottomRight)
            }
        },
        Some(s) => {
            // Try parsing "x, y" format
            let parts: Vec<&str> = s.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(x), Ok(y)) = (
                    parts[0].trim().parse::<f64>(),
                    parts[1].trim().parse::<f64>(),
                ) {
                    return_position_absolute(x, y)
                } else {
                    quote! {}
                }
            } else {
                quote! {}
            }
        }
        None => quote! {},
    };

    let default_impl = if has_any_pane_default {
        quote! {
            impl Default for #struct_name {
                fn default() -> Self {
                    Self {
                        #(#default_fields),*
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #default_impl

        impl saddle_pane::binding::PaneDerive for #struct_name {
            fn pane_title() -> &'static str {
                #title_str
            }

            fn field_descriptors() -> Vec<saddle_pane::binding::FieldDescriptor> {
                vec![#(#field_descriptors),*]
            }

            #position_impl

            fn read_field(&self, field_name: &str) -> Option<saddle_pane::controls::PaneValue> {
                match field_name {
                    #(#read_arms)*
                    _ => None,
                }
            }

            fn write_field(&mut self, field_name: &str, value: &saddle_pane::controls::PaneValue) -> bool {
                match field_name {
                    #(#write_arms)*
                    _ => false,
                }
            }
        }
    };

    expanded.into()
}

fn opt_f64(val: &Option<f64>) -> proc_macro2::TokenStream {
    match val {
        Some(v) => quote! { Some(#v) },
        None => quote! { None },
    }
}

fn opt_string(val: &Option<String>) -> proc_macro2::TokenStream {
    match val {
        Some(s) => quote! { Some(#s.to_string()) },
        None => quote! { None },
    }
}

#[derive(Default)]
struct FieldAttrs {
    skip: bool,
    label: Option<String>,
    folder: Option<String>,
    tab: Option<String>,
    tooltip: Option<String>,
    icon: Option<String>,
    order: i32,
    // Control type overrides
    force_slider: bool,
    force_number: bool,
    force_color: bool,
    force_monitor: bool,
    custom_control: Option<String>,
    custom_config: Vec<(String, f64)>,
    // Constraints
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    // Select options
    select_options: Option<Vec<String>>,
    // Default value for generated Default impl
    default_value: Option<Lit>,
}

fn parse_field_attrs(attr: &syn::Attribute, attrs: &mut FieldAttrs) {
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("skip") {
            attrs.skip = true;
        } else if meta.path.is_ident("slider") {
            attrs.force_slider = true;
        } else if meta.path.is_ident("number") {
            attrs.force_number = true;
        } else if meta.path.is_ident("color") {
            attrs.force_color = true;
        } else if meta.path.is_ident("monitor") {
            attrs.force_monitor = true;
        } else if meta.path.is_ident("label") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            if let Lit::Str(s) = lit {
                attrs.label = Some(s.value());
            }
        } else if meta.path.is_ident("folder") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            if let Lit::Str(s) = lit {
                attrs.folder = Some(s.value());
            }
        } else if meta.path.is_ident("tab") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            if let Lit::Str(s) = lit {
                attrs.tab = Some(s.value());
            }
        } else if meta.path.is_ident("tooltip") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            if let Lit::Str(s) = lit {
                attrs.tooltip = Some(s.value());
            }
        } else if meta.path.is_ident("icon") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            if let Lit::Str(s) = lit {
                attrs.icon = Some(s.value());
            }
        } else if meta.path.is_ident("order") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            if let Lit::Int(i) = lit {
                attrs.order = i.base10_parse()?;
            }
        } else if meta.path.is_ident("min") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            attrs.min = Some(lit_to_f64(&lit));
        } else if meta.path.is_ident("max") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            attrs.max = Some(lit_to_f64(&lit));
        } else if meta.path.is_ident("step") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            attrs.step = Some(lit_to_f64(&lit));
        } else if meta.path.is_ident("default") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            attrs.default_value = Some(lit);
        } else if meta.path.is_ident("custom") {
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            if let Lit::Str(s) = lit {
                attrs.custom_control = Some(s.value());
            }
        } else if meta.path.is_ident("select") {
            let mut options = Vec::new();
            meta.parse_nested_meta(|inner| {
                if inner.path.is_ident("options") {
                    let value = inner.value()?;
                    let array: syn::ExprArray = value.parse()?;
                    for elem in &array.elems {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: Lit::Str(s), ..
                        }) = elem
                        {
                            options.push(s.value());
                        }
                    }
                }
                Ok(())
            })?;
            attrs.select_options = Some(options);
        }
        Ok(())
    });
}

fn return_position_absolute(x: f64, y: f64) -> proc_macro2::TokenStream {
    quote! {
        fn pane_position() -> Option<saddle_pane::builder::PanePosition> {
            Some(saddle_pane::builder::PanePosition::Absolute(#x as f32, #y as f32))
        }
    }
}

/// Convert a literal to a token stream for use in a Default impl.
fn lit_to_default_expr(lit: &Lit) -> proc_macro2::TokenStream {
    match lit {
        Lit::Float(f) => {
            let val: f64 = f.base10_parse().unwrap_or(0.0);
            // Emit as untyped literal so Rust infers f32 or f64 from context
            let lit = proc_macro2::Literal::f64_unsuffixed(val);
            quote! { #lit }
        }
        Lit::Int(i) => {
            let val: i64 = i.base10_parse().unwrap_or(0);
            let lit = proc_macro2::Literal::i64_unsuffixed(val);
            quote! { #lit }
        }
        Lit::Bool(b) => {
            let val = b.value;
            quote! { #val }
        }
        Lit::Str(s) => {
            let val = s.value();
            quote! { #val.to_string() }
        }
        _ => quote! { Default::default() },
    }
}

fn lit_to_f64(lit: &Lit) -> f64 {
    match lit {
        Lit::Float(f) => f.base10_parse().unwrap_or(0.0),
        Lit::Int(i) => i.base10_parse::<i64>().unwrap_or(0) as f64,
        _ => 0.0,
    }
}

enum ControlKind {
    Slider {
        min: f64,
        max: f64,
        step: f64,
    },
    Number {
        min: Option<f64>,
        max: Option<f64>,
        step: f64,
    },
    Toggle,
    Text,
    Color,
    Select {
        options: Vec<String>,
    },
    Monitor,
    Custom {
        control_id: String,
        config: Vec<(String, f64)>,
    },
}

fn determine_control_kind(attrs: &FieldAttrs, field_type: &syn::Type) -> ControlKind {
    // Explicit overrides first
    if attrs.force_monitor {
        return ControlKind::Monitor;
    }
    if attrs.force_color {
        return ControlKind::Color;
    }
    if let Some(ref id) = attrs.custom_control {
        return ControlKind::Custom {
            control_id: id.clone(),
            config: attrs.custom_config.clone(),
        };
    }
    if let Some(ref options) = attrs.select_options {
        return ControlKind::Select {
            options: options.clone(),
        };
    }
    if attrs.force_number {
        return ControlKind::Number {
            min: attrs.min,
            max: attrs.max,
            step: attrs.step.unwrap_or(1.0),
        };
    }
    if attrs.force_slider || (attrs.min.is_some() && attrs.max.is_some()) {
        let min = attrs.min.unwrap_or(0.0);
        let max = attrs.max.unwrap_or(1.0);
        let step = attrs.step.unwrap_or(0.01);
        return ControlKind::Slider { min, max, step };
    }

    // Auto-detect from type
    let type_str = type_to_string(field_type);
    match type_str.as_str() {
        "bool" => ControlKind::Toggle,
        "String" => ControlKind::Text,
        "Color" | "Srgba" | "LinearRgba" | "Hsla" => ControlKind::Color,
        "f32" | "f64" => ControlKind::Number {
            min: attrs.min,
            max: attrs.max,
            step: attrs.step.unwrap_or(1.0),
        },
        "i32" | "i64" | "u32" | "u64" | "usize" | "isize" | "i8" | "i16" | "u8" | "u16" => {
            ControlKind::Number {
                min: attrs.min,
                max: attrs.max,
                step: attrs.step.unwrap_or(1.0),
            }
        }
        _ => ControlKind::Text,
    }
}

fn type_to_string(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(path) => {
            if let Some(segment) = path.path.segments.last() {
                segment.ident.to_string()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

/// Generate read_field and write_field match arms for a single field.
fn field_rw_arms(
    field_name_str: &str,
    field_ident: &proc_macro2::Ident,
    type_str: &str,
    kind: &ControlKind,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    // Determine PaneValue variant and conversion based on control kind + type
    let (read_arm, write_arm) = match kind {
        ControlKind::Slider { .. } | ControlKind::Number { .. } => {
            // Numeric types → PaneValue::Float(f64)
            let read = match type_str {
                "f64" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Float(self.#field_ident)), }
                }
                "f32" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Float(self.#field_ident as f64)), }
                }
                "i32" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Float(self.#field_ident as f64)), }
                }
                "i64" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Float(self.#field_ident as f64)), }
                }
                "u32" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Float(self.#field_ident as f64)), }
                }
                "u64" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Float(self.#field_ident as f64)), }
                }
                "usize" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Float(self.#field_ident as f64)), }
                }
                _ => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Float(self.#field_ident as f64)), }
                }
            };
            let write = match type_str {
                "f64" => {
                    quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Float(v) = value { self.#field_ident = *v; true } else { false } } }
                }
                "f32" => {
                    quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Float(v) = value { self.#field_ident = *v as f32; true } else { false } } }
                }
                "i32" => {
                    quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Float(v) = value { self.#field_ident = *v as i32; true } else { false } } }
                }
                "i64" => {
                    quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Float(v) = value { self.#field_ident = *v as i64; true } else { false } } }
                }
                "u32" => {
                    quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Float(v) = value { self.#field_ident = *v as u32; true } else { false } } }
                }
                "u64" => {
                    quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Float(v) = value { self.#field_ident = *v as u64; true } else { false } } }
                }
                "usize" => {
                    quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Float(v) = value { self.#field_ident = *v as usize; true } else { false } } }
                }
                _ => {
                    quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Float(v) = value { self.#field_ident = *v as f64; true } else { false } } }
                }
            };
            (read, write)
        }
        ControlKind::Toggle => {
            let read = quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Bool(self.#field_ident)), };
            let write = quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Bool(v) = value { self.#field_ident = *v; true } else { false } } };
            (read, write)
        }
        ControlKind::Text => {
            let read = quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::String(self.#field_ident.clone())), };
            let write = quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::String(v) = value { self.#field_ident.clone_from(v); true } else { false } } };
            (read, write)
        }
        ControlKind::Color => {
            let read = quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Color(self.#field_ident)), };
            let write = quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Color(v) = value { self.#field_ident = *v; true } else { false } } };
            (read, write)
        }
        ControlKind::Select { .. } => {
            // Select uses Int variant for index
            let read = match type_str {
                "usize" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Int(self.#field_ident as i64)), }
                }
                "i32" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Int(self.#field_ident as i64)), }
                }
                _ => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::Int(self.#field_ident as i64)), }
                }
            };
            let write = match type_str {
                "usize" => {
                    quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Int(v) = value { self.#field_ident = *v as usize; true } else { false } } }
                }
                "i32" => {
                    quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Int(v) = value { self.#field_ident = *v as i32; true } else { false } } }
                }
                _ => {
                    quote! { #field_name_str => { if let saddle_pane::controls::PaneValue::Int(v) = value { self.#field_ident = *v as usize; true } else { false } } }
                }
            };
            (read, write)
        }
        ControlKind::Monitor => {
            // Monitor fields are readable (game→pane) but not writable (pane→game).
            // Format the value as a string for display.
            let read = match type_str {
                "f32" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::String(format!("{:.2}", self.#field_ident))), }
                }
                "f64" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::String(format!("{:.2}", self.#field_ident))), }
                }
                "String" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::String(self.#field_ident.clone())), }
                }
                "bool" => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::String(format!("{}", self.#field_ident))), }
                }
                _ => {
                    quote! { #field_name_str => Some(saddle_pane::controls::PaneValue::String(format!("{}", self.#field_ident))), }
                }
            };
            let write = quote! { #field_name_str => false, };
            (read, write)
        }
        ControlKind::Custom { .. } => {
            let read = quote! { #field_name_str => None, };
            let write = quote! { #field_name_str => false, };
            (read, write)
        }
    };
    (read_arm, write_arm)
}

/// Convert "snake_case_name" to "Snake Case Name".
fn humanize_field_name(name: &str) -> String {
    name.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
