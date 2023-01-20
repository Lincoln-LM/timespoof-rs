use dll_syringe::{process::OwnedProcess, Syringe};

fn main() {
    let mut process_name = String::new();
    println!("Portion of Process Executable Name (ex. EmuHawk): ");
    std::io::stdin().read_line(&mut process_name).unwrap();

    let process_name = process_name.trim_end_matches("\r\n");
    let target_process =
        OwnedProcess::find_first_by_name(process_name).expect("Application not found running!");

    let syringe = Syringe::for_process(target_process);
    syringe
        .inject("./libtimespoof.dll")
        .expect("Failed to inject dll!");

    println!("DLL Injected");
}
