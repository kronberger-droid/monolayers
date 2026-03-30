#!/usr/bin/env nu

# Manage a local Nextcloud test instance via Docker.
# Usage:
#   nu scripts/test-nc.nu up      — start Nextcloud + create test config
#   nu scripts/test-nc.nu down    — stop and remove the container
#   nu scripts/test-nc.nu status  — check if the container is running

const CONTAINER = "monolayers-nc-test"
const PORT = 8080
const ADMIN_USER = "admin"
const ADMIN_PASS = "test123"
const SYNC_DIR = ($env.HOME | path join "nc-test")
const CONFIG_PATH = "config/test_config.toml"

def "main up" [] {
    # Check if already running
    let existing = (docker ps -a --filter $"name=($CONTAINER)" --format "{{.Status}}" | str trim)
    if ($existing | is-not-empty) {
        print $"Container '($CONTAINER)' already exists: ($existing)"
        print "Run 'nu scripts/test-nc.nu down' first to recreate."
        return
    }

    print "Starting Nextcloud container..."
    (docker run -d
        -p $"($PORT):80"
        -e $"NEXTCLOUD_ADMIN_USER=($ADMIN_USER)"
        -e $"NEXTCLOUD_ADMIN_PASSWORD=($ADMIN_PASS)"
        --name $CONTAINER
        nextcloud)

    # Create local sync directory with exempt folder
    mkdir $SYNC_DIR
    mkdir ($SYNC_DIR | path join "_working")

    # Create test config
    mkdir config
    let config = $"base_url = 'http://localhost:($PORT)/'

[user_credentials]
username = '($ADMIN_USER)'
password = '($ADMIN_PASS)'

local_sync_path = '($SYNC_DIR)'
exempt_folder_names = ['_working']
"
    $config | save -f $CONFIG_PATH

    print $"Nextcloud starting at http://localhost:($PORT)"
    print "Waiting for Nextcloud to be ready..."

    # Poll until Nextcloud responds
    mut ready = false
    for _ in 0..30 {
        let status = try { curl -s -o /dev/null -w "%{http_code}" $"http://localhost:($PORT)/status.php" } catch { "000" }
        if $status == "200" {
            $ready = true
            break
        }
        sleep 2sec
    }

    if $ready {
        print "Nextcloud is ready!"
        print $"Sync dir: ($SYNC_DIR)"
        print $"Config: ($CONFIG_PATH)"
        print ""
        print "Run 'cargo run' to start the daemon."
    } else {
        print "Nextcloud did not become ready within 60s."
        print $"Check with: docker logs ($CONTAINER)"
    }
}

def "main down" [] {
    print $"Stopping and removing '($CONTAINER)'..."
    try { docker rm -f $CONTAINER } catch { print "Container not found." }
    print "Done."
}

def "main status" [] {
    let result = (docker ps -a --filter $"name=($CONTAINER)" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | str trim)
    if ($result | is-empty) {
        print "No test container found."
    } else {
        print $result
    }
}

def main [] {
    print "Usage: nu scripts/test-nc.nu <up|down|status>"
}
