// Writes a file, adds to it, reads it back and removes it, through
// Std.Files and its StdioFiles adapter.
context Notes {
    publish { Keep }

    use { Files, FileError } from Std.Files
    use { to_bytes, to_text } from Std.Text

    service Keep(): Result<Unit, FileError> {
        calls Files, Console
    } = {
        files.write("notes.txt", "first line".to_bytes())?
        files.append("notes.txt", ", then more".to_bytes())?
        let text = files.read("notes.txt")?
        console.print(text.to_text())
        files.remove("notes.txt")?
        if files.exists("notes.txt") {
            console.print("still there")
        }
        files.read("notes.txt")?
        Unit
    }
}

domain Program {
    contexts { Notes }
    use { Keep } from Notes
} = Keep()?
