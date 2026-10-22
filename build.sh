#!/bin/bash

SHADER_IBIN_DIR="target/shaders"
OUT_PACKAGE_NAME="razed"

target=$([ "$1" != "windows" ] && echo 'linux' || echo 'windows')
target_rust=$([ "$1" != "windows" ] && echo 'x86_64-unknown-linux-gnu' || echo 'x86_64-pc-windows-gnu')
target_bin=$([ "$1" != "windows" ] && echo 'razed' || echo 'razed.exe')

echo "[*] Selected target platform: $target, Rust will compile with: $target_rust."

echo "[*] Compiling intermediate SPIR-V shader binaries..."
cargo run --bin shader_compiler --release

echo "[*] Compiling main application binary..."
cargo build --bin razed --release --target $target_rust

mkdir -p distribute/$target
cd distribute/$target

echo "[*] Creating package..."

echo "[+] Copying assets..."
mkdir razed
cp -rv ../../assets ./razed

echo "[+] Copying intermediate shader binaries..."
# only copy spir-v sources, glsl sources are not necessary
mkdir -p razed/target/shaders/spirv
cp -rv ../../target/shaders/spirv/* ./razed/target/shaders/spirv

echo "[+] Copying selected binary..."
cp -v ../../target/$target_rust/release/$target_bin ./razed

echo "[+] Compressing package..."
tar -czvf $OUT_PACKAGE_NAME-$target.tar.gz razed

echo "[+] Clearing..."
rm -rv razed

echo "[+] Package ready at: distribute/$target/$OUT_PACKAGE_NAME-$target.tar.gz" 

