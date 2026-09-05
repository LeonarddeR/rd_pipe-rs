fn main() {
	let time = std::time::Instant::now();

	windows_bindgen::bindgen(["--etc", "tools/bindgen/lib.txt"]);
	windows_bindgen::bindgen(["--etc", "tools/bindgen/tests.txt"]);

	println!("Finished in {:.2}s", time.elapsed().as_secs_f32());
}
