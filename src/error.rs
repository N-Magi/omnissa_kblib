use std::{error, string};

#[derive(Debug)]
pub enum Error{
    DownloadFailedExeption(String),
    ContentLoadingFailedExeption(String),
    KbPageDeserialzationFailed(String),
    JsonParsingFailedExeption(String),
    DataParsingFailedExeption(String),
    RequestSearchTokenFiledExeption(String),
    ObjectInitializationFailedExeption(String),
    SearchingFailedExeption(String),
    ObjectNotExistExeption(String)
}