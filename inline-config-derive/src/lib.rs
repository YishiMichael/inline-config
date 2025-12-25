mod config;
mod convert;
mod key;

fn delegate_macro<I, T>(f: fn(I) -> T, input: proc_macro::TokenStream) -> proc_macro::TokenStream
where
    I: syn::parse::Parse,
    T: quote::ToTokens,
{
    match syn::parse(input) {
        Ok(input) => f(input).into_token_stream().into(),
        Err(e) => proc_macro_error::abort!(e.span(), e),
    }
}

#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(std::convert::identity::<config::ConfigItems>, input)
}

#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn key(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(key::Key::expr, input)
}

#[proc_macro_error::proc_macro_error]
#[proc_macro]
#[allow(non_snake_case)]
pub fn Key(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(key::Key::ty, input)
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(ConfigData)]
pub fn config_data(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    delegate_macro(convert::config_data, input)
}

// use frunk::labelled;
// use frunk::labelled::Field;
// use frunk::{labelled::Transmogrifier, LabelledGeneric};
// fn f<S, T, I>(source: S) -> T
// where
//     S: Transmogrifier<T, I>,
// {
//     source.transmogrify()
// }

// struct MyFloat(f32);

// #[derive(LabelledGeneric)]
// struct __Repr {
//     ab: i32,
//     bc: MyFloat,
// }

// #[derive(LabelledGeneric)]
// struct Output {
//     ab: i32,
//     bc: f64,
// }

// impl Transmogrifier<(i32, u32), ()> for __Repr {
//     fn transmogrify(self) -> (i32, u32) {
//         (self.0, self.1 as u32)
//     }
// }

// impl<Key> Transmogrifier<f64, ()> for Field<Key, MyFloat> {
//     fn transmogrify(self) -> f64 {
//         self.value.0 as f64
//     }
// }

// fn a() {
//     let a: Output = f(__Repr {
//         ab: 6,
//         bc: MyFloat(7.0),
//     });
// }
