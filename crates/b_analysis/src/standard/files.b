// Std.Files: reading and writing whole files. A program imports what it
// needs with `use { Files, FileError } from Std.Files`, lists `Files` in
// `calls`, and names an adapter for it in b.toml: `StdioFiles` on machines
// with C's stdio, or one of its own, such as for a DivMMC.
context Files {
    publish { Files, FileError, NotFound, NoRoom, Unreadable, StdioFiles }

    // There is no file at that path.
    error NotFound

    // The disk is full, or the file holds more than a `Bytes` can.
    error NoRoom

    // The machine couldn't read or write it, for any other reason.
    error Unreadable

    enum FileError {
        include NotFound
        include NoRoom
        include Unreadable
    }

    port Files {
        // The whole file.
        fn read(path: String): Result<Bytes, FileError>

        // Replaces the file, or makes it.
        fn write(path: String, data: Bytes): Result<Unit, FileError>

        // Adds to the end of the file, or makes it.
        fn append(path: String, data: Bytes): Result<Unit, FileError>

        fn exists(path: String): Bool

        fn remove(path: String): Result<Unit, FileError>
    }

    // Files through C89's stdio: fopen, fread, fwrite and remove.
    adapter StdioFiles for Files = {
        fn read(path) = native b_files_read(path)
        fn write(path, data) = native b_files_write(path, data)
        fn append(path, data) = native b_files_append(path, data)
        fn exists(path) = native b_files_exists(path)
        fn remove(path) = native b_files_remove(path)
    }
}
