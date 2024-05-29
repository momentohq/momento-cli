use serde::Serialize;
use crate::commands::cloud_linter::api_gateway::ApiGatewayMetadata;

use crate::commands::cloud_linter::dynamodb::DynamoDbMetadata;
use crate::commands::cloud_linter::elasticache::ElastiCacheMetadata;
use crate::commands::cloud_linter::metrics::Metric;
use crate::commands::cloud_linter::s3::S3Metadata;
use crate::commands::cloud_linter::serverless_elasticache::ServerlessElastiCacheMetadata;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub(crate) enum Resource {
    ApiGateway(ApiGatewayResource),
    DynamoDb(DynamoDbResource),
    ElastiCache(ElastiCacheResource),
    ServerlessElastiCache(ServerlessElastiCacheResource),
    S3(S3Resource),
}

#[derive(Debug, Serialize, PartialEq)]
pub(crate) enum ResourceType {
    #[serde(rename = "AWS::ApiGateway::API")]
    ApiGateway,
    #[serde(rename = "AWS::DynamoDB::GSI")]
    DynamoDbGsi,
    #[serde(rename = "AWS::DynamoDB::Table")]
    DynamoDbTable,
    #[serde(rename = "AWS::Elasticache::RedisNode")]
    ElastiCacheRedisNode,
    #[serde(rename = "AWS::Elasticache::MemcachedNode")]
    ElastiCacheMemcachedNode,
    #[serde(rename = "AWS::Elasticache::Serverless")]
    ServerlessElastiCache,
    #[serde(rename = "AWS::S3::Bucket")]
    S3,
}

#[derive(Serialize, Debug)]
pub(crate) struct DynamoDbResource {
    #[serde(rename = "type")]
    pub(crate) resource_type: ResourceType,
    pub(crate) region: String,
    pub(crate) id: String,
    pub(crate) metrics: Vec<Metric>,
    #[serde(rename = "metricPeriodSeconds")]
    pub(crate) metric_period_seconds: i32,
    pub(crate) metadata: DynamoDbMetadata,
}

#[derive(Serialize, Debug)]
pub(crate) struct ElastiCacheResource {
    #[serde(rename = "type")]
    pub(crate) resource_type: ResourceType,
    pub(crate) region: String,
    pub(crate) id: String,
    pub(crate) metrics: Vec<Metric>,
    #[serde(rename = "metricPeriodSeconds")]
    pub(crate) metric_period_seconds: i32,
    pub(crate) metadata: ElastiCacheMetadata,
}

#[derive(Serialize, Debug)]
pub(crate) struct ServerlessElastiCacheResource {
    #[serde(rename = "type")]
    pub(crate) resource_type: ResourceType,
    pub(crate) region: String,
    pub(crate) id: String,
    pub(crate) metrics: Vec<Metric>,
    #[serde(rename = "metricPeriodSeconds")]
    pub(crate) metric_period_seconds: i32,
    pub(crate) metadata: ServerlessElastiCacheMetadata,
}

#[derive(Serialize, Debug)]
pub(crate) struct S3Resource {
    #[serde(rename = "type")]
    pub(crate) resource_type: ResourceType,
    pub(crate) region: String,
    pub(crate) id: String,
    pub(crate) metrics: Vec<Metric>,
    #[serde(rename = "metricPeriodSeconds")]
    pub(crate) metric_period_seconds: i32,
    pub(crate) metadata: S3Metadata,
}

#[derive(Serialize, Debug)]
pub(crate) struct ApiGatewayResource {
    #[serde(rename = "type")]
    pub(crate) resource_type: ResourceType,
    pub(crate) region: String,
    pub(crate) id: String,
    pub(crate) metrics: Vec<Metric>,
    #[serde(rename = "metricPeriodSeconds")]
    pub(crate) metric_period_seconds: i32,
    pub(crate) metadata: ApiGatewayMetadata,
}
