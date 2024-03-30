use encoding_rs::SHIFT_JIS;
use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::io::{BufReader, Seek, SeekFrom, Write};

fn main() -> io::Result<()> {
    // ログファイルと位置情報ファイルのパス
    let log_path = "log.txt";
    let position_path = "position.txt";

    // 位置情報ファイルを開き、前回の終了位置を取得
    let mut last_position = 0;
    if let Ok(mut position_file) = File::open(position_path) {
        let mut position_string = String::new();
        position_file.read_to_string(&mut position_string)?;
        last_position = position_string.parse::<u64>().unwrap_or(0);
    }

    // ログファイルを開き、前回の終了位置から読み込みを開始
    let log_file = File::open(log_path)?;
    let mut log_stream = BufReader::new(log_file);
    log_stream.seek(SeekFrom::Start(last_position))?;

    // 出力ファイルを開く
    let output_path = "output_utf8.txt";
    let mut output_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)?;

    // ログファイルの末尾を監視し、新しい行が追加されるのを待つ
    loop {
        let mut buf = Vec::new();
        match log_stream.read_to_end(&mut buf) {
            Ok(0) => {
                // EOFに到達したら少し待ってから再試行
                dbg!("EOF reached, waiting...");
                std::thread::sleep(std::time::Duration::from_millis(1000));
                continue;
            }
            Ok(bytes_read) => {
                // 文字コードを変換して出力
                let (cow, _encoding_used, _had_errors) = SHIFT_JIS.decode(&buf);
                let utf8_line = cow.into_owned();
                dbg!(&utf8_line);
                output_file.write_all(utf8_line.as_bytes())?;

                // 現在の位置を更新
                last_position += bytes_read as u64;

                // 現在の位置を位置情報ファイルに保存
                let mut position_file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(position_path)?;
                write!(position_file, "{}", last_position)?;
            }
            Err(e) => {
                dbg!("Error reading line:", &e);
                return Err(e);
            }
        }
    }
}
