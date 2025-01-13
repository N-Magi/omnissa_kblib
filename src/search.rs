use std::{iter::Filter, os::raw};

use chrono::{DateTime, Utc};
use serde_json::Value;
use tokio::{self, net::tcp::ReuniteError};
use super::error;

struct SearchResult{
    total_count:i64,
    total_count_filterd:i64,
    //search_uid:&'a str,
    kb_items:Vec<ResultItem>
}

struct ResultItem {
    title:String,
    click_uri:String,
    kb_num:String,
}

#[derive(Clone)]
struct SearchClient{
    token:String
}

struct SearchFilter<'a>{
    start_date:Option<DateTime<Utc>>,
    end_date:Option<DateTime<Utc>>,
    language:Option<&'a str>,
    timezone:Option<&'a str>,
    number_of_results:i64
}

impl Default for SearchFilter<'_> {
    fn default() -> Self {
        Self { 
            start_date: None, 
            end_date: None, 
            language: Some("English"), 
            timezone: Some("Asia/Tokyo") ,
            number_of_results:10
        }
    }
}


impl SearchClient {

    pub async fn new() -> Result<Self,super::error::Error> {      
        let token = SearchClient::get_token().await?;       
        Ok(SearchClient {
            token:token
        })
    }

    async fn get_token() -> Result<String,super::error::Error>{

        let url="https://kb.omnissa.com/s/sfsites/aura?other.CommunitySearchExternal.getSearchToken=1";
        let form = [
            ("aura.pageURI","/s/global-search/"),
            ("aura.context","{\"mode\":\"PROD\",\"app\":\"siteforce:communityApp\",\"loaded\":{\"APPLICATION@markup://siteforce:communityApp\":\"1183_iYPVTlE11xgUFVH2RcHXYA\",\"COMPONENT@markup://instrumentation:o11ySecondaryLoader\":\"342_x7Ue1Ecg1Vom9Mcos08ZPw\"},\"dn\":[],\"globals\":{},\"uad\":false}"),
            ("message", "{\"actions\":[{\"id\":\"42;a\",\"descriptor\":\"apex://CommunitySearchExternalController/ACTION$getSearchToken\",\"callingDescriptor\":\"markup://c:CommunitySearchEndpointHandler\",\"params\":{}}]}"),
            ("aura.token","null")
        ];

        let client = reqwest::Client::new();
        let raw_response = client.post(url).header("Content-Type", "application/x-www-form-urlencoded").form(&form).send().await.map_err(|f| super::error::Error::RequestSearchTokenFiledExeption(f.to_string()))?;
        let raw_result = raw_response.text().await.map_err(|f| super::error::Error::DownloadFailedExeption(f.to_string()))?;

        let json_result:Value = serde_json::from_str(&raw_result).map_err(|f| super::error::Error::ContentLoadingFailedExeption(f.to_string()))?;
        let Some(raw_return_value) = json_result["actions"][0]["returnValue"].as_str() else {
            return Err(super::error::Error::DataParsingFailedExeption("Json Scheme is Invalid".to_string()));
        };

        let return_value:Value = serde_json::from_str(raw_return_value).map_err(|f| super::error::Error::JsonParsingFailedExeption(f.to_string()))?;
        let Some(token) = return_value["token"].as_str() else {
            return Err(super::error::Error::ObjectNotExistExeption("token is not exist".to_string()));
        };
        return Ok(token.to_string());

    }


    pub async fn search(self,filter:SearchFilter<'_>) -> Result<SearchResult,super::error::Error>{

        let url = "https://platform.cloud.coveo.com/rest/search/v2";
        
        let start_date = filter.start_date.map(|s_date| s_date.format("%Y/%m/%d").to_string());
        let end_date = filter.end_date.map(|s_date| s_date.format("%Y/%m/%d").to_string());
        let laungage = filter.language.unwrap_or_default();
        let time_zone = filter.timezone.unwrap_or_default();
        
        let mut aq = String::new();
        //日付入力用の処理
        if start_date.is_some(){
            aq += &format!("(@commondate >={}",start_date.clone().unwrap());

            if end_date.is_some() {
                aq += " AND ";
            } else {
                aq += ")";
            }
        } 
        if end_date.is_some() {
            if start_date.is_none() {
                aq += "(";
            }
            aq += &format!("@commondate <={})",end_date.unwrap());
        }

        //言語フィルタ用の処理
        aq += &format!("(@commonlanguage=={})",laungage);
        

        let form = [
            ("q","".to_string()),
            ("aq",aq.clone()),
            ("timezone",time_zone.to_string()),
            ("numberOfResults",filter.number_of_results.to_string()),
            ("firstResult","0".to_string())
            ];


        let mut result = self.post_serch(url, &form).await?;
        
        //すでに一回分取得しているため -1する
        let page = (result.total_count / filter.number_of_results - 1);

        //残りのページ分の結果を取得する
        for i in 1..page {
            let first_result = i * filter.number_of_results;

            let form = [
            ("q","".to_string()),
            ("aq",aq.clone()),
            ("timezone",time_zone.to_string()),
            ("numberOfResults",filter.number_of_results.to_string()),
            ("firstResult", first_result.to_string())
            ];

            let mut page =  self.post_serch(url, &form).await?;

            result.kb_items.append(&mut page.kb_items);
        }
        Ok(
            result
        )
    }

    async fn post_serch(&self,url:&str,form:&[(&str,String)]) -> Result<SearchResult,super::error::Error>{

        let client = reqwest::Client::new();
        let raw_result = client.post(url)
        //.bearer_auth(format!("Bearer {}",self.token))
        .header("Authorization", format!("Bearer {}",self.token))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(form).send().await
        .map_err(|f| super::error::Error::SearchingFailedExeption(f.to_string()))?;

        let result = raw_result.text().await.map_err(|f| super::error::Error::DataParsingFailedExeption(f.to_string()))?;
        let json_result:Value = serde_json::from_str(&result).map_err(|f| super::error::Error::JsonParsingFailedExeption(f.to_string()))?;
        
        let Some(total_count) = json_result["totalCount"].as_i64() else {
            return Err(super::error::Error::ObjectNotExistExeption("total_count params is not Exist".to_string()));
        };

        let Some(total_count_filterd) = json_result["totalCountFiltered"].as_i64() else {
            return Err(super::error::Error::ObjectNotExistExeption("total_count_Filterd params is not Exist".to_string()));
        };

        let Some(kb_results) = json_result["results"].as_array() else {
            return Err(super::error::Error::ObjectNotExistExeption("results params is not Exist".to_string()));
        };


        let mut kb_items:Vec<ResultItem> = Vec::new();

        for kb_result in kb_results {

            let Some(title) = kb_result["title"].as_str() else {
                return Err(super::error::Error::ObjectNotExistExeption("title is no Exits".to_string()));
            };

            let Some(click_uri) = kb_result["clickUri"].as_str() else {
                return Err(super::error::Error::ObjectNotExistExeption("clickUri is no Exits".to_string()));
            };

            let Some(kb_num) = kb_result["raw"]["sfurlname"].as_str() else {
                return Err(super::error::Error::ObjectNotExistExeption("sfurlname is no Exits".to_string()));
            };

            kb_items.push(ResultItem {
                title:title.to_string(),
                click_uri:click_uri.to_string(),
                kb_num: kb_num.to_string()
            });
        }


        Ok(
            SearchResult{
                total_count:total_count,
                total_count_filterd:total_count_filterd,
                kb_items:kb_items
            }
        )
        
    }   


}

mod test {
    #[tokio::test()]
    async fn test() {
        
        let s_client = crate::search::SearchClient::new().await.unwrap();
        let mut filter = crate::search::SearchFilter::default();
        filter.number_of_results = 200;
        let rrr = s_client.search(filter).await.unwrap();

        println!("total count:{}",rrr.total_count);
        
        for r in rrr.kb_items {
            println!("title:{}",r.title);
            println!("kb_num:{}",r.kb_num);
        }
    }
}
