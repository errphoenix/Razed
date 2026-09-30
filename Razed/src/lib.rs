mod assets;
mod data;
mod procedural;
mod render;
mod state;
mod structure;
mod ui;

#[macro_export]
macro_rules! compile_const {
    (
        $p:vis const $n:ident: $t:ty = $v:literal;
    ) => {
        $p const $n: $t = $v;

        paste::paste! {
            #[macro_export]
            macro_rules! [< const_ $n:lower _ $t:lower >] {
                () => { $v };
            }
        }
    };
}
