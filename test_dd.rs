use neonote::notes::NoteDocument;
fn main() {
    let mut doc = NoteDocument::default();
    doc.content = "line1\nline2\nline3".to_string();
    doc.enter_normal();
    doc.handle_normal_input("d");
    doc.handle_normal_input("d");
    println!("Content: {:?}", doc.content());
}
