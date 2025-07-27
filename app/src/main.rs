use voice_input::Recorder;

fn main() {
    let mut recorder = Recorder::new();
    let input_list = recorder.get_inputs();

    if let Err(e) = input_list {
        eprintln!("{}", e);
        return;
    }

    let input_list = input_list.unwrap();

    println!("Available Inputs:");
    input_list.iter().for_each(|input| {
        println!("  {input}");
    });

    let input = recorder.set_input(None);
    if let Err(e) = input {
        eprintln!("{}", e);
    }

    println!("Chose input: {}", recorder.get_input_name().unwrap())
}
