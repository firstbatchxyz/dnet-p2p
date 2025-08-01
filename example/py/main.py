from time import sleep
from socket import gethostname
from src.dnet_p2p import DnetP2P
from secrets import token_hex
import sys


def main():
    is_manager = "-m" in sys.argv
    instance = token_hex(8)
    hostname = gethostname()
    address = "<dont-care>"

    if is_manager:
        print(f"Starting manager {instance} at {hostname}")
    else:
        print(f"Starting worker {instance} at {hostname}")

    with DnetP2P("../../target/release") as dnet:
        dnet.enable_logs()  # enables rust logging
        dnet.create_instance(instance, hostname, address, is_manager=is_manager)
        dnet.start()

        # sleep a bit
        print("Dnet service started. Press Ctrl+C to stop.")
        try:
            while True:
                if is_manager:
                    properties = dnet.get_properties()
                    print(f"Properties: {properties}")
                sleep(1)
        except KeyboardInterrupt:
            print("Stopping dnet service...")
    pass


if __name__ == "__main__":
    main()
