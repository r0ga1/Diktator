fn main() {
    let a = std::env::args().collect::<Vec<_>>();
    assert_eq!(a.len(), 3, "input output");
    let pcm = diktator::audio::read_file(std::path::Path::new(&a[1])).unwrap();
    diktator::audio::write_wav(std::path::Path::new(&a[2]), &pcm).unwrap();
}
