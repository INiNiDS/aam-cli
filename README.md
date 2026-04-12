# **AAM CLI**

A powerful and intuitive CLI for working with AAM (Abstract Alias Mapping) files, powered by the aam-rs library.

## **Features**

### **Quick Access Commands**

* **aam check <file>** – Validate an AAM file for errors.
* **aam format <file>** – Format an AAM file.
* **aam get <file> <key>** – Retrieve a value by its key.
* **aam [files...]** — Launch the interactive TUI editor.
* **aam lsp** – Start the LSP server.

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

| Key       | Action                              |
|:----------|:------------------------------------|
| Ctrl+T    | Check file                          |
| Ctrl+F    | Format file                         |
| Ctrl+S    | Save file                           |
| Ctrl+Q    | Quit                                |
| Ctrl+H    | Toggle help                         |
| Tab       | Switch focus (Editor ↔ Input field) |
| Ctrl+-->  | Next tab                            |
| Ctrl+ <-- | Previous Tab                        |
| Enter     | Execute command from input field    |
| Esc       | Close help window                   |

#### **Command Line Inputs**

You can type commands directly into the TUI input field:

* check – Check file
* format – Format file
* save or w – Save file
* get <key> – Get value by key
* help – Show help
* quit – Exit

## **Installation**
## Via bash
```bash
curl -fsSL https://raw.githubusercontent.com/ininids/aam-cli/main/install.sh | sh
```
## AUR
Install with your favorite AUR helper:
```bash
# Coming soon
paru -S aam-cli
```
## Via Homebrew
```bash
brew install ininids/tap/aam
```
## Via Cargo 
To install the CLI tool globally:
```bash
cargo install aam
```

## From source:  
```bash
git clone https://github.com/ininids/aam-cli
cd aam-cli  
cargo install --path .
```
## **Usage Examples**

### **Validating a File**

aam check config.aam

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
aam format config.aam

\# Format in-place  
aam format config.aam \--inplace

### **Launching TUI**

\# With one file  
aam tui config.aam

\# With multiple files  
aam tui config.aam database.aam schema.aam

### **Running the LSP Server**

aam lsp

## **Dependencies**

* aam-rs – Core library for AAM file processing.
* ratatui – Modern TUI framework for Rust.
* crossterm – Cross-platform terminal manipulation.
* clap – Command-line argument parsing.
* tui-textarea – Text area widget for the TUI.
* tower-lsp – Language Server Protocol implementation.

## **Project Structure**

src/  
├── main.rs \# Main entry point, CLI commands  
├── tui.rs \# TUI interface, animations, and multi-file logic  
└── lsp.rs \# Language Server Protocol implementation

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