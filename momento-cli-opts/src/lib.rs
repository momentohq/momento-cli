use std::error::Error;

use clap::CommandFactory;
use clap::Parser;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum LoginMode {
    Browser,
    Qr,
}

#[derive(Debug, Parser)]
#[command(
    version,
    bin_name = "momento",
    name = "momento",
    about = "Command line tool for Momento Serverless Cache"
)]
pub struct Momento {
    #[arg(name = "verbose", global = true, long, help = "Log more information")]
    pub verbose: bool,

    #[arg(
        long,
        short,
        default_value = "default",
        global = true,
        help = "User profile"
    )]
    pub profile: String,

    #[command(subcommand)]
    pub command: Subcommand,
}

impl Momento {
    pub fn meta_command() -> clap::Command {
        Momento::command()
    }
}

#[derive(Debug, Parser)]
pub enum Subcommand {
    #[command(about = "Interact with caches")]
    Cache {
        #[arg(
            long = "endpoint",
            short = 'e',
            global = true,
            help = "An explicit hostname to use; for example, cell-us-east-1-1.prod.a.momentohq.com"
        )]
        endpoint: Option<String>,

        #[command(subcommand)]
        operation: CacheCommand,
    },
    #[command(
        about = "Interact with topics",
        before_help = "
These commands requires a cache, which serves as a namespace
for your topics. If you haven't already, call `cache create`
to make one!

To create a topic, subscribe to it.
To delete a topic, stop subscribing to it."
    )]
    Topic {
        #[arg(
            long = "endpoint",
            short = 'e',
            global = true,
            help = "An explicit hostname to use; for example, cell-us-east-1-1.prod.a.momentohq.com"
        )]
        endpoint: Option<String>,

        #[command(subcommand)]
        operation: TopicCommand,
    },
    #[command(about = "Configure credentials")]
    Configure {
        #[arg(long, short)]
        quick: bool,
        #[arg(
            long,
            short,
            help = "Overwrite credentials by providing an api key v2 and endpoint"
        )]
        api_key_and_endpoint: bool,
        #[arg(
            long,
            short,
            help = "Overwrite credentials by providing a disposable auth token or legacy v1 api key"
        )]
        disposable_token: bool,
    },
    #[command(about = "Manage accounts", hide = true)]
    Account {
        #[command(subcommand)]
        operation: AccountCommand,
    },
    #[command(
        about = "**PREVIEW** features which are in beta. Feedback is welcome!",
        hide = true
    )]
    Preview {
        #[command(subcommand)]
        operation: PreviewCommand,
    },
}

const SIGNUP_DEPRECATED_MSG: &str =
    "*DECOMMISSIONED* Please go to the Momento Console (https://console.gomomento.com) to sign up.";

#[derive(Debug, Parser)]
pub enum AccountCommand {
    #[command(about = SIGNUP_DEPRECATED_MSG)]
    Signup {
        // We've kept these subcommands as options so that if someone calls one of them
        // they get a helpful error message to go to the console.
        #[command(subcommand)]
        signup_operation: Option<CloudSignupCommand>,
    },
}

#[derive(Debug, Parser)]
pub enum FunctionCommand {
    #[command(about = "Create or update a Momento Function")]
    PutFunction {
        #[arg(long = "cache-name", short, help = "Cache namespace")]
        cache_name: String,
        #[arg(long = "name", short, help = "Function name")]
        name: String,
        #[arg(
            long = "wasm-file",
            short,
            help = ".wasm file compiled with wasm32-wasip2"
        )]
        wasm_file: Option<String>,
        #[arg(
            long = "id-uploaded-wasm",
            short,
            help = "ID of a Wasm binary previously uploaded to Momento Functions"
        )]
        id_uploaded_wasm: Option<String>,
        #[arg(
            long = "version-uploaded-wasm",
            short,
            help = "Version number of a Wasm binary previously uploaded to Momento Functions"
        )]
        version_uploaded_wasm: Option<u32>,
        #[arg(long = "description", short, help = "Description")]
        description: Option<String>,
        #[arg(
            long = "env-var",
            short = 'E',
            value_parser = parse_env::<String, String>,
            help = "Environment variables to provide to the Function. Example: -E KEY1=value_1 -E KEY2=value_2"
        )]
        environment_variables: Vec<(String, String)>,
    },
    #[command(about = "Create or update a wasm source that can be used in a Momento Function")]
    PutWasm {
        #[arg(long = "name", short, help = "Wasm source name")]
        name: String,
        #[arg(
            long = "wasm-file",
            short,
            help = ".wasm file compiled with wasm32-wasip2"
        )]
        wasm_file: String,
        #[arg(long = "description", short, help = "Description")]
        description: Option<String>,
    },
    #[command(about = "Call a Momento Function")]
    InvokeFunction {
        #[arg(long = "id", short, help = "Function ID")]
        function_id: String,
    },
    #[command(about = "List all Momento Functions in the given cache namespace")]
    ListFunctions {
        #[arg(long = "cache-name", short, help = "Cache namespace")]
        cache_name: String,
    },
    #[command(about = "List all versions of a Momento Function")]
    ListFunctionVersions {
        #[arg(long = "id", short, help = "Function ID")]
        function_id: String,
    },
    #[command(about = "List all wasm sources")]
    ListWasms {},
}

#[derive(Debug, Parser)]
pub enum PreviewCommand {
    #[command(
        about = "**PREVIEW** Query your AWS account to find optimizations with Momento",
        before_help = "
!!                                                                !!
!!                        Preview feature                         !!
!!   For more information, contact us at support@gomomento.com.   !!
!!                                                                !!

This command will be used to fetch information about your Elasticache clusters and DynamoDB tables
to help find opportunities for optimizations with Momento.
"
    )]
    CloudLinter {
        #[arg(long, short, help = "The AWS region to examine")]
        region: String,
        #[arg(
            long = "enable-ddb-ttl-check",
            help = "Opt in to check whether ddb tables have ttl enabled. If there are lots of tables, this could slow down data collection"
        )]
        enable_ddb_ttl_check: bool,
        #[arg(
            long = "enable-gsi",
            help = "Opt in to check metrics on dynamodb gsi's. If there are lots of tables with gsi's, this could slow down data collection"
        )]
        enable_gsi: bool,
        #[arg(
            long = "enable-s3",
            help = "Opt in to check metrics on s3. If there are lots of s3 buckets, this could slow down data collection"
        )]
        enable_s3: bool,
        #[arg(
            long = "enable-api-gateway",
            help = "Opt in to check metrics on API Gateway"
        )]
        enable_api_gateway: bool,
        #[arg(
            value_enum,
            long = "resource",
            help = "Pass in a specific resource type to only collect data on that resource. Example: --resource dynamo"
        )]
        resource: Option<CloudLinterResources>,
        #[arg(
            long = "metric-collection-rate",
            help = "tps at which to invoke the aws `get-metric-data` api",
            default_value = "10"
        )]
        metric_collection_rate: u32,
        #[arg(
            long = "start-date",
            help = "The inclusive UTC start date of the metric collection period. Will use (end-date - 30 days) if not provided. (YYYY-MM-DD)"
        )]
        metric_start_date: Option<String>,
        #[arg(
            long = "end-date",
            help = "The inclusive UTC end date of the metric collection period. Will use the current date if not provided. (YYYY-MM-DD)"
        )]
        metric_end_date: Option<String>,
    },
    #[command(about = "**PREVIEW** Create or update your Momento Functions")]
    Function {
        #[command(subcommand)]
        operation: FunctionCommand,
    },
}

#[derive(clap::ValueEnum, PartialEq, Eq, Debug, Clone, Copy)]
pub enum CloudLinterResources {
    ApiGateway,
    S3,
    Dynamo,
    ElastiCache,
    ElastiCacheRedis,
    ElastiCacheMemcached,
    ElastiCacheServerless,
    ElastiCacheValkey,
}

#[derive(Debug, Parser)]
pub enum CloudSignupCommand {
    #[command(about = SIGNUP_DEPRECATED_MSG)]
    Gcp {
        #[arg(long, short)]
        email: Option<String>,
        #[arg(long, short)]
        region: Option<String>,
    },
    #[command(about = SIGNUP_DEPRECATED_MSG)]
    Aws {
        #[arg(long, short)]
        email: Option<String>,
        #[arg(long, short)]
        region: Option<String>,
    },
}

#[derive(Debug, Parser)]
pub enum CacheCommand {
    #[command(
    about = "Create a cache",
    group(
    clap::ArgGroup::new("cache-name")
    .required(true)
    .args(["cache_name", "cache_name_flag", "cache_name_flag_for_backward_compatibility"]),
    ),
    )]
    Create {
        #[arg(
            help = "Name of the cache you want to create. Must be at least 3 characters and unique within your account.",
            value_name = "CACHE"
        )]
        cache_name: Option<String>,

        #[arg(long = "cache", value_name = "CACHE")]
        cache_name_flag: Option<String>,
        #[arg(long = "name", value_name = "CACHE")]
        cache_name_flag_for_backward_compatibility: Option<String>,
    },

    #[command(
    about = "Delete a cache",
    group(
    clap::ArgGroup::new("cache-name")
    .required(true)
    .args(["cache_name", "cache_name_flag", "cache_name_flag_for_backward_compatibility"]),
    ),
    )]
    Delete {
        #[arg(help = "Name of the cache you want to delete.", value_name = "CACHE")]
        cache_name: Option<String>,

        #[arg(long = "cache", value_name = "CACHE")]
        cache_name_flag: Option<String>,
        #[arg(long = "name", value_name = "CACHE")]
        cache_name_flag_for_backward_compatibility: Option<String>,
    },

    #[command(about = "List all caches")]
    List {},

    #[command(about = "Flush all contents from a cache",
group(
clap::ArgGroup::new("cache-name")
.required(true)
.args(["cache_name", "cache_name_flag"])))]
    Flush {
        #[arg(help = "Name of the cache to flush.", value_name = "CACHE")]
        cache_name: Option<String>,

        #[arg(long = "cache", value_name = "CACHE")]
        cache_name_flag: Option<String>,
    },

    #[command(
    about = "Store an item in a cache",
    group(
    clap::ArgGroup::new("cache-key")
    .required(true)
    .args(["key", "key_flag"]),
    ),
    group(
    clap::ArgGroup::new("cache-value")
    .required(true)
    .args(["value", "value_flag"]),
    ),
    group(
    clap::ArgGroup::new("cache-name")
    .args(["cache_name", "cache_name_flag_for_backward_compatibility"]),
    ),
    )]
    Set {
        #[arg(
            long = "cache",
            help = "Name of the cache you want to use. If not provided, your profile's default cache is used.",
            value_name = "CACHE"
        )]
        cache_name: Option<String>,
        #[arg(long = "name", value_name = "CACHE")]
        cache_name_flag_for_backward_compatibility: Option<String>,

        // TODO: Add support for non-string key-value
        #[arg(help = "Cache key under which to store the value")]
        key: Option<String>,
        #[arg(long = "key", value_name = "KEY")]
        key_flag: Option<String>,

        #[arg(help = "Cache value to store under the key. This will be stored as UTF-8 bytes.")]
        value: Option<String>,
        #[arg(long = "value", value_name = "VALUE")]
        value_flag: Option<String>,

        #[arg(
            long = "ttl",
            help = "Max time, in seconds, that the item will be stored in cache"
        )]
        ttl_seconds: Option<u64>,
    },

    #[command(
    about = "Get an item from the cache",
    group(
    clap::ArgGroup::new("cache-key")
    .required(true)
    .args(["key", "key_flag"]),
    ),
    group(
    clap::ArgGroup::new("cache-name")
    .args(["cache_name", "cache_name_flag_for_backward_compatibility"]),
    ),
    )]
    Get {
        #[arg(
            long = "cache",
            help = "Name of the cache you want to use. If not provided, your profile's default cache is used.",
            value_name = "CACHE"
        )]
        cache_name: Option<String>,
        #[arg(long = "name", value_name = "CACHE")]
        cache_name_flag_for_backward_compatibility: Option<String>,

        // TODO: Add support for non-string key-value
        #[arg(help = "Cache key under which to store the value")]
        key: Option<String>,
        #[arg(long = "key", value_name = "KEY")]
        key_flag: Option<String>,
    },

    #[command(
    about = "Delete an item from the cache",
    group(
    clap::ArgGroup::new("cache-key")
    .required(true)
    .args(["key", "key_flag"]),
    ),
    group(
    clap::ArgGroup::new("cache-name")
    .args(["cache_name", "cache_name_flag_for_backward_compatibility"]),
    ),
    )]
    DeleteItem {
        #[arg(
            long = "cache",
            help = "Name of the cache you want to use. If not provided, your profile's default cache is used.",
            value_name = "CACHE"
        )]
        cache_name: Option<String>,
        #[arg(long = "name", value_name = "CACHE")]
        cache_name_flag_for_backward_compatibility: Option<String>,

        // TODO: Add support for non-string key-value
        #[arg(help = "Cache key to delete")]
        key: Option<String>,
        #[arg(long = "key", value_name = "KEY")]
        key_flag: Option<String>,
    },
}

#[derive(Debug, Parser)]
pub enum TopicCommand {
    /// Publish a value to all subscribers of a topic.
    #[command()]
    Publish {
        #[arg(
            long = "cache",
            help = "Name of the cache you want to use as your topic namespace. If not provided, your profile's default cache is used.",
            value_name = "CACHE"
        )]
        cache_name: Option<String>,

        #[arg(help = "Name of the topic to which you would like to publish")]
        topic: String,
        #[arg(help = "String message value to publish")]
        value: String,
    },

    /// Subscribe to messages coming in on a topic.
    #[command()]
    Subscribe {
        #[arg(
            long = "cache",
            help = "Name of the cache you want to use as your topic namespace. If not provided, your profile's default cache is used.",
            value_name = "CACHE"
        )]
        cache_name: Option<String>,

        #[arg(help = "Name of the topic to which you would like to subscribe")]
        topic: String,
    },
}

fn parse_env<K, V>(s: &str) -> Result<(K, V), Box<dyn Error + Send + Sync + 'static>>
where
    K: std::str::FromStr,
    K::Err: Error + Send + Sync + 'static,
    V: std::str::FromStr,
    V::Err: Error + Send + Sync + 'static,
{
    let equals = s
        .find('=')
        .ok_or_else(|| format!("invalid environment variable syntax: no `=` found in `{s}`"))?;
    let key = s[..equals].parse()?;
    let value = s[equals + 1..].parse()?;
    Ok((key, value))
}
