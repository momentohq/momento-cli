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
    },
    #[command(about = "Manage accounts")]
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

#[derive(Debug, Parser)]
pub enum SigningKeyCommand {
    #[command(about = "Create a signing key")]
    Create {
        #[arg(
            long = "ttl",
            short = 't',
            default_value = "86400",
            help = "Duration, in minutes, that the signing key will be valid"
        )]
        ttl_minutes: u32,
    },

    #[command(about = "Revoke the signing key")]
    Revoke {
        #[arg(long = "key-id", short, help = "Signing Key ID")]
        key_id: String,
    },

    #[command(about = "List all signing keys")]
    List {},
}

#[derive(Debug, Parser)]
pub enum AccountCommand {
    #[command(about = "Sign up for Momento")]
    Signup {
        #[command(subcommand)]
        signup_operation: CloudSignupCommand,
    },
}

#[derive(Debug, Parser)]
pub enum PreviewCommand {
    #[command(
        about = "**PREVIEW** Manage signing keys",
        before_help = "
!!                                                                !!
!!                        Preview feature                         !!
!!   For more information, contact us at support@gomomento.com.   !!
!!                                                                !!

Signing keys can be used to generate pre-signed URLs that allow access to a value in the cache
for a specified period of time.  The URLs can be distributed to browsers or devices to allow
them to access the cache value without having access to a Momento auth token.
    "
    )]
    SigningKey {
        #[arg(
            long = "endpoint",
            short = 'e',
            global = true,
            help = "An explicit hostname to use; for example, cell-us-east-1-1.prod.a.momentohq.com"
        )]
        endpoint: Option<String>,

        #[command(subcommand)]
        operation: SigningKeyCommand,
    },
    #[command(
        about = "**PREVIEW** Log in to manage your Momento account",
        before_help = "
!!                                                                !!
!!                        Preview feature                         !!
!!   For more information, contact us at support@gomomento.com.   !!
!!                                                                !!

This command will be used to log in to your Momento account and generate secure, time-limited
tokens for accessing your Momento caches.
"
    )]
    Login {
        #[arg(value_enum, default_value = "browser")]
        via: LoginMode,
    },
    #[command(
        group(
            clap::ArgGroup::new("generate-token")
                .required(true)
                .args(&["valid_for", "never_expire"]),
        ),
        about = "**PREVIEW** Generate an api token for Momento",
        before_help = "
!!                                                                !!
!!                        Preview feature                         !!
!!   For more information, contact us at support@gomomento.com.   !!
!!                                                                !!

This command will be used to generate api tokens to use with Momento. If `--never-expire` is specified,
then the generated token will never expire. Else, it will expire after the specified number of seconds.
Either `--valid-for` or `--never-expire` must be specified.
"
    )]
    GenerateToken {
        #[arg(
            long,
            short,
            help = "Specify how long until the token expires. ex. 1d, 10m, 2M"
        )]
        valid_for: Option<String>,
        #[arg(long, help = "Generate a token that never expires")]
        never_expire: bool,
        #[arg(
            long = "endpoint",
            short = 'e',
            global = true,
            help = "An explicit hostname to use; for example, cell-us-east-1-1.prod.a.momentohq.com"
        )]
        endpoint: Option<String>,
    },
}

#[derive(Debug, Parser)]
pub enum CloudSignupCommand {
    #[command(about = "Signup for Momento on GCP")]
    Gcp {
        #[arg(long, short)]
        email: String,
        #[arg(long, short, value_name = "us-east1 or asia-northeast1")]
        region: String,
    },
    #[command(about = "Signup for Momento on AWS")]
    Aws {
        #[arg(long, short)]
        email: String,
        #[arg(
            long,
            short,
            value_name = "us-west-2, us-east-1, ap-northeast-1, ap-south-1"
        )]
        region: String,
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

    #[command(about = "Flush all contents from a cache")]
    Flush {
        #[arg(help = "Name of the cache to flush.", value_name = "CACHE")]
        cache_name: String,
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
