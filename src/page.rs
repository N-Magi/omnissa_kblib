
use serde_json::Value;
use tokio;
use reqwest;
use std::{str::FromStr, time};
use chrono::{self, DateTime, TimeZone, Utc,NaiveDateTime};
use super::error;
use crate::page;

struct Page{
    kb_num:i64,
    create_date:chrono::DateTime<Utc>,
    last_modified_date:chrono::DateTime<Utc>,
    title:String,
    content:String,
}

pub struct PageClient{

}
impl PageClient {

    pub async fn get_content(skb_num:String) -> Result<Page,super::error::Error>{
        
        //let skb_num = kb_num.to_string();

        let url="https://kb.omnissa.com/s/sfsites/aura?r=18&aura.ApexAction.execute=1";
        let form = [
            ("aura.pageURI",format!("/s/article/{}?lang=en_US",skb_num)),
            ("aura.context","{\"mode\":\"PROD\",\"app\":\"siteforce:communityApp\",\"loaded\":{\"APPLICATION@markup://siteforce:communityApp\":\"1183_iYPVTlE11xgUFVH2RcHXYA\",\"COMPONENT@markup://instrumentation:o11ySecondaryLoader\":\"342_x7Ue1Ecg1Vom9Mcos08ZPw\"},\"dn\":[],\"globals\":{},\"uad\":false}".to_string()),
            ("message", "{ \"actions\":[{\"id\":\"376;a\",\"descriptor\":\"aura://ApexActionController/ACTION$execute\",\"callingDescriptor\":\"UNKNOWN\",
            \"params\":{\"namespace\":\"\",\"classname\":\"KM_iKBArticleDetailsController\",\"method\":\"getArticleDetails\",
            \"params\":{\"documentId\":\"".to_string() + &skb_num + "\",\"language\":\"en_US\",\"isInternal\":false},
            \"cacheable\":true,\"isContinuation\":false}}]}"),
            ("aura.token","null".to_string())
        ];
        
        let client = reqwest::Client::new();
        let result = client.post(url)
        .header("Content-Type","application/x-www-form-urlencoded")
        .form(&form).send().await.map_err(|f| {super::error::Error::DownloadFailedExeption(f.to_string())})?;

        let a = result.text().await.map_err(|f| super::error::Error::ContentLoadingFailedExeption(f.to_string()))?;

        let instance = PageClient::deserialize(a)?;
        return Ok(instance);
    }

    pub fn deserialize(raw_content:String) -> Result<Page,super::error::Error>{
        let json:Value = serde_json::from_str(&raw_content).map_err(|f| super::error::Error::JsonParsingFailedExeption(f.to_string()))?;
        
        let Some(json_inner_content) = json["actions"][0]["returnValue"]["returnValue"].as_str() else {
            return Err(super::error::Error::DataParsingFailedExeption("Json Parsing Error".to_string()));
        };

        let inner_json:Value = serde_json::from_str(&json_inner_content).map_err(|f| super::error::Error::JsonParsingFailedExeption(f.to_string()))?;

        let Some(title) = inner_json["meta"]["articleInfo"]["title"].as_str() else {
            return Err(super::error::Error::DataParsingFailedExeption("title json parsing error".to_string()));
        };

        let Some(str_create_date) = inner_json["meta"]["articleInfo"]["createdDate"].as_str() else {
            return Err(super::error::Error::DataParsingFailedExeption("createDate json parsing error".to_string()));
        };
        let create_date = NaiveDateTime::parse_from_str(str_create_date, "%Y-%m-%d %H:%M:%S")
            .map_err(|f| super::error::Error::DataParsingFailedExeption(f.to_string()))?;

        let Some(str_last_update) = inner_json["meta"]["articleInfo"]["lastModifiedDate"].as_str() else {
            return Err(super::error::Error::DataParsingFailedExeption("last update json parsing error".to_string()));
        };
        let last_modified_date = NaiveDateTime::parse_from_str(str_last_update, "%Y-%m-%d %H:%M:%S")
            .map_err(|f| super::error::Error::DataParsingFailedExeption(f.to_string()))?;


        let Some(contents) = inner_json["content"].as_array() else {
            return Err(super::error::Error::DataParsingFailedExeption("Getting Kb Contents".to_string()));
        };

        let mut contnt:String = String::new();
        for raw_content in contents{
            let Some(cnt) = raw_content.as_str() else {
                continue;
            };
            contnt += cnt;
        }

        return Ok(Page {
            kb_num:0,
            create_date:create_date.and_utc(),
            last_modified_date:last_modified_date.and_utc(),
            title: title.to_string(),
            content: contnt,
        });

    }
}

#[cfg(test)]
mod Test{
    use super::PageClient;


    #[tokio::test]
    async fn test(){
        let page = PageClient::get_content("97771".to_string()).await.unwrap();
        println!("{}",page.title);
    }
}