use clap::Parser;

mod text_processor;



#[derive(Parser)]
struct Args {
    #[arg(long)]
    template: String,
    #[arg(long)]
    out: String,

    #[arg(long, default_value="")]
    active_tags: String
}



fn main() {
    let args = Args::parse();

    let settings = text_processor::Settings {
        active_tags: args.active_tags.split(',').map(|slice| slice.to_string()).collect()
    };

    let text = std::fs::read_to_string(args.template).unwrap();
    let text = text_processor::Processor::process_text(&text, &settings);
    std::fs::write(args.out, text.iter().collect::<String>()).unwrap();
}

