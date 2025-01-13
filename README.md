# Command Guardian

This is the command for preventing the user from unexpected SIGINT signal. This command confirms the user's intention.

![demo](./demo/demo.gif)

## Usecase

If you are executing a command that takes a long time to complete, such as training a machine learning model, and you want to prevent accidentally stopping the command by pressing `Ctrl+C`, you can use this command.

## Installation

Now, only arm64 and x86_64 architectures are supported.

```sh
git clone https://github.com/SNKK62/command-guardian.git

cd command-guardian

cp bin/cmgd_$(arch) <your_path>/cmgd (e.g. /usr/local/bin)
```

And add to PATH (in most cases, you have alerady added `/usr/local/bin` to PATH).

## Usage

```sh
cmgd <command>
```

### Example

```sh
cmgd python script.py
```

## Demo


