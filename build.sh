#!/bin/bash

SHADER_IBIN_DIR="target/shaders"
OUT_PACKAGE_NAME="razed"

target=$([ "$1" != "windows" ] && echo 'linux' || echo 'windows')
target_rust=$([ "$1" != "windows" ] && echo 'x86_64-unknown-linux-gnu' || echo 'x86_64-pc-windows-gnu')
target_bin=$([ "$1" != "windows" ] && echo 'razed' || echo 'razed.exe')

mark_info=$(echo -e "\e[48;2;0;71;171m \e[38;2;255;255;255;1mi \e[;0m")
mark_rust=$(echo -e "\e[48;5;172m \e[38;2;255;255;255;1mR \e[;0m")
mark_pkg=$(echo -e "\e[48;5;125m \e[38;2;255;255;255;1m# \e[;0m")
style_minor=$(echo -e "\e[38;5;244m")
mark_end_success=$(echo -e "\e[48;5;28m \e[38;2;255;255;255;1m! \e[;0m")

echo "$mark_info Selected target platform: $target; will compile with: $target_rust."

echo "$mark_rust Compiling intermediate SPIR-V shader binaries..."
cargo run --bin shader_compiler --release 2>/dev/null

echo "$mark_rust Compiling main application binary..."
cargo build --bin razed --release --target $target_rust -F default,$BUILD_FEATURES 2>/dev/null

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

echo "$mark_pkg $style_minor Copying selected binary..."
cp ../../target/$target_rust/release/$target_bin ./razed

echo "$mark_pkg Compressing package..."
tar -czf $OUT_PACKAGE_NAME-$target.tar.gz razed

echo "$mark_info Clearing..."
rm -r razed

echo "$mark_end_success Package ready at: distribute/$target/$OUT_PACKAGE_NAME-$target.tar.gz" 

