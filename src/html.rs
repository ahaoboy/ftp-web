use bytesize::ByteSize;
use fileinfo::FileInfo;
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};
use suppaftp::FtpStream;

pub const HTML_STYLE: &str = r#"
    :root {
        --bg-color: #fff;
        --text-color: #222;
        --link-color: #0366d6;
        --link-visited-color: #f22526;
        --dir-icon-color: #79b8ff;
        --file-icon-color: #959da5;
    }
    body {background: var(--bg-color); color: var(--text-color);}
    a {text-decoration:none;color:var(--link-color);}
    a:visited {color: var(--link-visited-color);}
    a:hover {text-decoration:underline;}
    header a {padding: 0 6px;}
    footer {text-align:center;font-size:12px;}
    table {text-align:left;border-collapse: collapse;}
    tr {border-bottom: solid 1px #ccc;}
    tr:last-child {border-bottom: none;}
    th, td {padding: 5px;}
    th {text-align: center;}
    th:first-child,td:first-child {text-align: center;}
    svg[data-icon="dir"] {vertical-align: text-bottom; color: var(--dir-icon-color); fill: currentColor;}
    svg[data-icon="file"] {vertical-align: text-bottom; color: var(--file-icon-color); fill: currentColor;}
    svg[data-icon="home"] {width:18px;}
    @media (prefers-color-scheme: dark) {
        :root {
            --bg-color: #222;
            --text-color: #ddd;
            --link-color: #539bf5;
            --link-visited-color: #f25555;
            --dir-icon-color: #7da3d0;
            --file-icon-color: #545d68;
        }
    }"#;
pub const DIR_ICON: &str = r#"<svg aria-label="Directory" data-icon="dir" width="20" height="20" viewBox="0 0 512 512" version="1.1" role="img"><path fill="currentColor" d="M464 128H272l-64-64H48C21.49 64 0 85.49 0 112v288c0 26.51 21.49 48 48 48h416c26.51 0 48-21.49 48-48V176c0-26.51-21.49-48-48-48z"></path></svg>"#;
pub const FILE_ICON: &str = r#"<svg aria-label="File" data-icon="file" width="20" height="20" viewBox="0 0 384 512" version="1.1" role="img"><path d="M369.9 97.9L286 14C277 5 264.8-.1 252.1-.1H48C21.5 0 0 21.5 0 48v416c0 26.5 21.5 48 48 48h288c26.5 0 48-21.5 48-48V131.9c0-12.7-5.1-25-14.1-34zM332.1 128H256V51.9l76.1 76.1zM48 464V48h160v104c0 13.3 10.7 24 24 24h104v288H48z"/></svg>"#;
pub const HOME_ICON: &str = r#"<svg aria-hidden="true" data-icon="home" viewBox="0 0 576 512"><path fill="currentColor" d="M280.37 148.26L96 300.11V464a16 16 0 0 0 16 16l112.06-.29a16 16 0 0 0 15.92-16V368a16 16 0 0 1 16-16h64a16 16 0 0 1 16 16v95.64a16 16 0 0 0 16 16.05L464 480a16 16 0 0 0 16-16V300L295.67 148.26a12.19 12.19 0 0 0-15.3 0zM571.6 251.47L488 182.56V44.05a12 12 0 0 0-12-12h-56a12 12 0 0 0-12 12v72.61L318.47 43a48 48 0 0 0-61 0L4.34 251.47a12 12 0 0 0-1.6 16.9l25.5 31A12 12 0 0 0 45.15 301l235.22-193.74a12.19 12.19 0 0 1 15.3 0L530.9 301a12 12 0 0 0 16.9-1.6l25.5-31a12 12 0 0 0-1.7-16.93z"></path></svg>"#;

pub fn get_html(ftp: &Arc<Mutex<FtpStream>>, path: &str) -> String {
    let path = path.replace("//", "/");
    let mut ftp = ftp.lock().unwrap();

    let (list, path) = if let Ok(list) = ftp.list(Some(&path)) {
        (list, path)
    } else {
        ((ftp.list(None)).unwrap(), "".into())
    };

    let li_text = list.iter().filter_map(|i| {
        let info: FileInfo = FileInfo::from_str(i).ok()?;
        let name = &info.name;
        let time = &info.last_modified;
        let icon: &str = if info.is_dir() { DIR_ICON } else { FILE_ICON };

        if info.is_dir() {
            return Some(format!(
                r#"
<tr>
  <td>{icon}</td>
  <td><a href="/ftp/{path}/{name}/">{name}</a></td>
  <td>{time}</td>
  <td></td>
</tr>
                "#,
            ));
        }

        let size = ByteSize(info.size.try_into().unwrap());
        Some(format!(
            r#"
<tr>
  <td>{icon}</td>
  <td><a download href="/file/{path}/{name}">{name}</a></td>
  <td>{time}</td>
  <td style='text-align: right'>{size}</td>
</tr>
      "#
        ))
    });

    let parent_path = path.trim_end_matches('/');
    let mut parent_path = parent_path.split('/').collect::<Vec<_>>();
    if parent_path.len() > 1 {
        parent_path.pop();
    }

    let parent_path = parent_path.join("/");
    let parent_dir = format!(
        r#"
<tr>
  <td>{DIR_ICON}</td>
  <td><a href="/ftp/{parent_path}/">..</a></td>
  <td></td>
  <td></td>
</tr>
"#
    );

    let li_text = vec![parent_dir]
        .into_iter()
        .chain(li_text)
        .collect::<Vec<_>>();

    let li_text = li_text.join("\n");

    let path = if path.is_empty() { "/".into() } else { path };

    fn header_links(path: &str) -> String {
        let segments = path
            .trim_start_matches('/')
            .trim_end_matches('/')
            .split('/');
        let mut link = "".to_string();
        format!(
            r#"<a href="/">{}</a>{}"#,
            HOME_ICON,
            segments
                .map(|seg| {
                    link = format!("{link}/{seg}");
                    format!("/<a href='/ftp/{link}'>{seg}</a>")
                })
                .collect::<Vec<_>>()
                .join("")
        )
    }

    let link = header_links(&path);
    let html = format!(
        r#"
<html>
  <style>
  {HTML_STYLE}
  </style>

  <head>
    <meta name="viewport" content="width=device-width">
    <meta charset='utf-8'>
    <title>Index of {path}</title>
  </head>
  <header>
    <h3>
      Index of: {link}
    </h3>
  </header>
  <hr />

    <body>
      <table>
        <tr>
          <th></th>
          <th>Name</th>
          <th>Last modified</th>
          <th>Size</th>
        </tr>
        {li_text}
      </table>

      <hr/>

      <footer>
        <a href="https://github.com/ahaoboy/ftp-web" target="_blank">ftp-web</a>
      </footer>
    </body>
</html>"#
    );

    html
}
