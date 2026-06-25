# Setx
一个 Windows 环境变量管理工具

## Build

>Require rust toolchain

```sh
git clone https://github.com/AzureZee/setx
cd setx
cargo build --release
```

## Usage

```sh
setx <var-name> [value]              设置变量；不带 value 则删除该变量
setx [-a | -append]     <paths...>   将路径追加到 PATH 末尾
setx [-p | -prepend]    <paths...>   将路径加到 PATH 开头
setx [-d | -delete]     <paths...>   从 PATH 中移除指定路径
setx [-e | -edit-path]  <editor>     用编辑器打开 PATH，保存后写回
setx [-h | -help]                    显示帮助
```

## Example

```sh
setx JAVA_HOME 'C:\Program Files\Java\jdk-21'
setx -a '%USERPROFILE%\bin' 'C:\tools'
setx -p 'C:\Program Files\Go\bin'
setx -d 'C:\old\tool'
setx -e code
```
## License

[MIT](LICENSE.txt)
