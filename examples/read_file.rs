use noise_gui::Expr;

fn main() {
    let mut test_expr: Expr = ron::from_str(include_str!("test.ron")).unwrap();

    // This gets a noise value which is identical to what was designed in the GUI
    println!("Sampled value: {}", test_expr.noise().get([0.0, 0.0, 0.0]));

    // This replaces the value of a variable and samples a new noise function:
    println!(
        "Updated value: {}",
        test_expr
            .set_f64("my-var", 42.0)
            .noise()
            .get([0.0, 0.0, 0.0])
    );
}
