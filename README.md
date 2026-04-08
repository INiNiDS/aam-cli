# **AAM CLI**

A powerful and intuitive CLI for working with AAM (Abstract Alias Mapping) files, powered by the aam-rs library.

## **Features**

### **Quick Access Commands**

* **aam-cli check \<file\>** – Validate an AAM file for errors.
* **aam-cli format \<file\>** – Format an AAM file.
* **aam-cli get \<file\> \<key\>** – Retrieve a value by its key.
* **aam-cli tui \[files...\]** – Launch the interactive TUI editor.
* **aam-cli lsp** – Start the LSP server.

### **TUI (Terminal User Interface)**

A modern TUI built with ratatui, featuring a rich set of capabilities:

#### **Design**

* **Branded Header** with animated "A A M" lettering and a running progress bar.
* **"INiNiDS" branding** and the current CLI version displayed in the header.
* **Animated Input Field** with a gray background and a white running bar.
* **File Window (Top)** – Displays the contents of the AAM file.
* **Command Line (Bottom)** – For entering commands directly.

#### **Multi-file Support**

* Open multiple files simultaneously.
* Tab system for seamless switching between files.
* Scalable interface that adapts to the number of open files.

#### **Hotkeys**

| Key | Action |
| :---- | :---- |
| Ctrl+T | Check file |
| Ctrl+F | Format file |
| Ctrl+S | Save file |
| Ctrl+Q | Quit |
| Ctrl+H | Toggle help |
| Tab | Switch focus (Editor ↔ Input field) |
| Ctrl+Tab | Next tab |
| Enter | Execute command from input field |
| Esc | Close help window |

#### **Command Line Inputs**

You can type commands directly into the TUI input field:

* check or c – Check file
* format, fmt, or f – Format file
* save or w – Save file
* get \<key\> or g \<key\> – Get value by key
* help or h – Show help
* quit or q – Exit

## **Installation**

To install the CLI tool globally:  
cargo install \--path .

Or install it from source:  
git clone \[https://github.com/ininids/aam-cli\](https://github.com/ininids/aam-cli)  
cd aam-cli  
cargo install \--path .

## **Usage Examples**

### **Validating a File**

aam-cli check config.aam

**Output:**  
✓ config.aam successfully validated  
Keys found: 3  
Schemas found: 1  
Types found: 7

### **Getting a Value by Key**

aam-cli get config.aam host

**Output:**  
localhost

### **Formatting a File**

\# Output to stdout  
aam-cli format config.aam

\# Format in-place  
aam-cli format config.aam \--inplace

### **Launching TUI**

\# With one file  
aam-cli tui config.aam

\# With multiple files  
aam-cli tui config.aam database.aam schema.aam

### **Running the LSP Server**

aam-cli lsp

## **Dependencies**

* aam-rs – Core library for AAM file processing.
* ratatui – Modern TUI framework for Rust.
* crossterm – Cross-platform terminal manipulation.
* clap – Command-line argument parsing.
* tui-textarea – Text area widget for the TUI.
* tower-lsp – Language Server Protocol implementation.

## **Project Structure**

src/  
├── main.rs    \# Main entry point, CLI commands  
├── tui.rs     \# TUI interface, animations, and multi-file logic  
└── lsp.rs     \# Language Server Protocol implementation

## **Implementation Details**

### **Animations**

* **Running Progress Bar** – Moves across the input field on a 4-second cycle.
* **"A A M" Letter Highlighting** – Each letter pulses individually with a deceleration effect.
* **Fade-in/Fade-out Effects** – Smooth transitions for intensity and UI elements.

### **Color Scheme**

* **Dark Background** (RGB 20–40) for eye comfort.
* **Gray Input Field** (RGB 35–40) with a high-contrast white progress bar.
* **Status Indicators** – Green (Success), Red (Error), Yellow (Info).

### **Scalability**

* The interface automatically adapts to the number of files.
* Tabs only appear when two or more files are open.
* All elements scale dynamically based on terminal size.

## **License**
See the LICENSE file for details.

## **Community**

* Contribution guide: [Contributing](.github/CONTRIBUTING.md)
* Security policy: [Security](.github/SECURITY.md)
* Code of conduct: [Code of Conduct](.github/CODE_OF_CONDUCT.md)
* Issue templates: [Issue Template](.github/ISSUE_TEMPLATE)
* Pull request template: [Pull Request Template](.github/pull_request_template.md)

## **Author**

**INiNiDS** – [aam.ininids.in.rs](https://aam.ininids.in.rs/)