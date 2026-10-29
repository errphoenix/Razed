#!/bin/bash

SHADER_IBIN_DIR="target/shaders"
OUT_PACKAGE_NAME="razed"

target=$([ "$1" != "windows" ] && echo 'linux' || echo 'windows')
target_rust=$([ "$1" != "windows" ] && echo 'x86_64-unknown-linux-gnu' || echo 'x86_64-pc-windows-gnu')
target_ext=$([ "$1" != "windows" ] && echo '' || echo '.exe')

mark_info=$(echo -e "\e[48;2;0;71;171m \e[38;2;255;255;255;1mi \e[;0m")
mark_rust=$(echo -e "\e[48;5;172m \e[38;2;255;255;255;1mR \e[;0m")
mark_pkg=$(echo -e "\e[48;5;125m \e[38;2;255;255;255;1m# \e[;0m")
style_minor=$(echo -e "\e[38;5;244m")
mark_end_success=$(echo -e "\e[48;5;28m \e[38;2;255;255;255;1m! \e[;0m")

echo "$mark_info Selected target platform: $target; will compile with: $target_rust."

echo "$mark_rust Compiling intermediate SPIR-V shader binaries..."
cargo run --bin shader_compiler --release 2>/dev/null

rm -r build
mkdir build

# todo: info in bin dir
# todo: select executable

bin_build() {
    local name=$1
    local cpu=$2
    mkdir "build/$name"
    echo "$mark_rust Compiling application binary for feature target $cpu ($name)"
    RUSTFLAGS="-C target-cpu=$cpu"
    cargo build --bin razed --release --target $target_rust -F default,$BUILD_FEATURES --target-dir "build/$name" 2>/dev/null
}

bin_cpy() {
    local name=$1
    cp "../../build/$name/$target_rust/release/razed$target_ext" ./razed
    mv razed/razed$target_ext razed/bin/razed-$name$target_ext
}

bin_build "sse2" "x86-64"
bin_build "sse4" "x86-64-v2"
bin_build "avx2" "x86-64-v3"
bin_build "avx512" "x86-64-v4"

mkdir -p distribute/$target
cd distribute/$target

echo "$mark_pkg Creating package..."

echo "$mark_pkg $style_minor Copying assets..."
mkdir razed
cp -r ../../assets ./razed

echo "$mark_pkg $style_minor Copying intermediate shader binaries..."
# only copy spir-v sources, glsl sources are not necessary
mkdir -p razed/target/shaders/spirv
cp -r ../../target/shaders/spirv/* ./razed/target/shaders/spirv

echo "$mark_pkg $style_minor Copying binaries..."
mkdir razed/bin
bin_cpy "sse2"
bin_cpy "sse4"
bin_cpy "avx2"
bin_cpy "avx512"

echo "$mark_pkg Compressing package..."
tar -czf $OUT_PACKAGE_NAME-$target.tar.gz razed

echo "$mark_info Clearing..."
rm -r razed

echo "$mark_end_success Package ready at: distribute/$target/$OUT_PACKAGE_NAME-$target.tar.gz"
