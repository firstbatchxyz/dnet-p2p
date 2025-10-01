from time import sleep
from socket import gethostname
from src.dnet_p2p import DnetP2P
from secrets import token_hex
import sys


def main():
    print("Running dnet-p2p example")
    is_manager = "-m" in sys.argv
    is_passive = "-p" in sys.argv
    instance = token_hex(12)
    hostname = gethostname()

    if is_passive:
        print(f"Starting passive monitor {instance} at {hostname}")
    elif is_manager:
        print(f"Starting manager {instance} at {hostname}")
    else:
        print(f"Starting worker {instance} at {hostname}")

    with DnetP2P("../../target/release") as dnet:
        dnet.enable_logs()  # enables rust logging
        dnet.create_instance(
            instance,
            hostname,
            "localhost",  # host
            8080,  # server_port
            50501,  # shard_port
            is_manager=is_manager,
            is_passive=is_passive,
        )
        dnet.start()

        # sleep a bit
        print("Dnet service started. Press Ctrl+C to stop.")
        try:
            while True:
                if is_manager:
                    properties = dnet.get_properties()
                    print(f"Properties: {properties}")
                sleep(5)
        except KeyboardInterrupt:
            print("Stopping dnet service...")
    pass


if __name__ == "__main__":
    main()
