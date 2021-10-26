# distream

distreamは長時間の音声録音Botで、2時間以上の録音が可能です。

音声はwebmで保存され、ユーザーごとにトラックが分離されているので書き出す際にユーザーを選択できます。

### how to use

`!join` to join voice channel and start recording.

`!leave` to leave voice channel and stop recording.

recorded voice is saved on file system.

### how to host

You need to install Rust.

```bash
$ git clone https://github.com/virtualCrypto-discord/distream.git
$ cd distream
$ DISCORD_BOT_TOKEN=<your bot token here> cargo run
```

### contributors

[sizumita](https://github.com/sizumita) \
[tignear](https://github.com/tignear) \
[Shirataki2](https://github.com/Shirataki2)
