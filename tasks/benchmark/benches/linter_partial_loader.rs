use std::fmt::Write;

use oxc_benchmark::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use oxc_linter::loader::PartialLoader;

/// A Svelte component whose template renders `n` web-component tags.
///
/// Custom element names must contain a `-`, so names that start with `script-` (such as
/// `<script-icon>`) are perfectly valid -- and every one of them is a `<script` candidate that
/// the partial loader's `find_script_start` has to classify and reject. The `.vue` and `.astro`
/// loaders share that function, so Svelte is representative of all three.
///
/// Note that the real `<script>` block sitting at the top does not make the rest of the file
/// free: a Svelte file may contain a second `<script module>` block, so after extracting the
/// first one the loader scans the whole remaining template looking for it.
fn svelte_with_n_custom_elements(n: usize) -> String {
    let mut source = String::with_capacity(64 * n + 256);
    source.push_str("<script lang=\"ts\">\n");
    source.push_str("  export let title: string;\n");
    source.push_str("  export let size: number = 24;\n");
    source.push_str("</script>\n\n");
    source.push_str("<!-- generated icon gallery -->\n");
    source.push_str("<h1>{title}</h1>\n<ul>\n");
    for i in 0..n {
        writeln!(source, r#"  <li><script-icon name="icon-{i}" size={{size}} /></li>"#).unwrap();
    }
    source.push_str("</ul>\n");
    source
}

fn bench_linter_partial_loader(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("linter_partial_loader");

    // `n` quadruples at each step, so the source size quadruples too. Reporting throughput
    // makes the scaling behaviour visible at a glance: a linear loader extracts at a flat
    // MiB/s regardless of `n`, while anything super-linear shows a throughput that collapses
    // as `n` grows.
    for n in [256_usize, 1024, 4096] {
        let source_text = svelte_with_n_custom_elements(n);
        group.throughput(Throughput::Bytes(source_text.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &source_text, |b, source_text| {
            b.iter(|| PartialLoader::parse("svelte", source_text));
        });
    }

    group.finish();
}

criterion_group!(linter_partial_loader, bench_linter_partial_loader);
criterion_main!(linter_partial_loader);
