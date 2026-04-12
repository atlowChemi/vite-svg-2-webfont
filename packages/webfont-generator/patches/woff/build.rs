fn main() {
    #[cfg(feature = "version2")]
    let target = std::env::var("TARGET").unwrap();

    #[cfg(feature = "version1")]
    {
        let mut build = cc::Build::new();
        build
            .include("vendor/sfnt2woff/source/woff")
            .file("vendor/sfnt2woff/source/woff/woff.c")
            .warnings(false);

        // Use zlib headers from libz-sys instead of system zlib
        if let Some(include) = std::env::var_os("DEP_Z_INCLUDE") {
            for path in std::env::split_paths(&include) {
                build.include(path);
            }
        }

        build.compile("libsfnt2woff.a");
    }

    #[cfg(feature = "version2")]
    cc::Build::new()
        .cpp(true)
        .flag("-std=c++11")
        .file("vendor/woff2/wrapper/woff2.cpp")
        .include("vendor/woff2/source/include")
        .include("vendor/woff2/wrapper")
        .warnings(false)
        .compile("libwoff2wrapper.a");

    #[cfg(feature = "version2")]
    cc::Build::new()
        .cpp(true)
        .flag("-std=c++11")
        .include("vendor/brotli/source/c/include")
        .include("vendor/woff2/source/include")
        .file("vendor/woff2/source/src/font.cc")
        .file("vendor/woff2/source/src/glyph.cc")
        .file("vendor/woff2/source/src/normalize.cc")
        .file("vendor/woff2/source/src/table_tags.cc")
        .file("vendor/woff2/source/src/transform.cc")
        .file("vendor/woff2/source/src/variable_length.cc")
        .file("vendor/woff2/source/src/woff2_common.cc")
        .file("vendor/woff2/source/src/woff2_dec.cc")
        .file("vendor/woff2/source/src/woff2_enc.cc")
        .file("vendor/woff2/source/src/woff2_out.cc")
        .warnings(false)
        .compile("libwoff2.a");

    #[cfg(feature = "version2")]
    cc::Build::new()
        .include("vendor/brotli/source/c/include")
        .file("vendor/brotli/source/c/common/constants.c")
        .file("vendor/brotli/source/c/common/context.c")
        .file("vendor/brotli/source/c/common/dictionary.c")
        .file("vendor/brotli/source/c/common/platform.c")
        .file("vendor/brotli/source/c/common/shared_dictionary.c")
        .file("vendor/brotli/source/c/common/transform.c")
        .file("vendor/brotli/source/c/dec/bit_reader.c")
        .file("vendor/brotli/source/c/dec/decode.c")
        .file("vendor/brotli/source/c/dec/huffman.c")
        .file("vendor/brotli/source/c/dec/state.c")
        .file("vendor/brotli/source/c/enc/backward_references.c")
        .file("vendor/brotli/source/c/enc/backward_references_hq.c")
        .file("vendor/brotli/source/c/enc/bit_cost.c")
        .file("vendor/brotli/source/c/enc/block_splitter.c")
        .file("vendor/brotli/source/c/enc/brotli_bit_stream.c")
        .file("vendor/brotli/source/c/enc/cluster.c")
        .file("vendor/brotli/source/c/enc/command.c")
        .file("vendor/brotli/source/c/enc/compound_dictionary.c")
        .file("vendor/brotli/source/c/enc/compress_fragment.c")
        .file("vendor/brotli/source/c/enc/compress_fragment_two_pass.c")
        .file("vendor/brotli/source/c/enc/dictionary_hash.c")
        .file("vendor/brotli/source/c/enc/encode.c")
        .file("vendor/brotli/source/c/enc/encoder_dict.c")
        .file("vendor/brotli/source/c/enc/entropy_encode.c")
        .file("vendor/brotli/source/c/enc/fast_log.c")
        .file("vendor/brotli/source/c/enc/histogram.c")
        .file("vendor/brotli/source/c/enc/literal_cost.c")
        .file("vendor/brotli/source/c/enc/memory.c")
        .file("vendor/brotli/source/c/enc/metablock.c")
        .file("vendor/brotli/source/c/enc/static_dict.c")
        .file("vendor/brotli/source/c/enc/utf8_util.c")
        .warnings(false)
        .compile("libbrotli.a");

    #[cfg(feature = "version2")]
    if target == "x86_64-unknown-linux-musl" {
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=pthread");
    }
}
