from time import sleep
from socket import gethostname
from src.dnet_p2p import DnetP2P
from secrets import token_hex
import sys


def main():
    is_manager = "-m" in sys.argv
    instancename = token_hex(8)
    hostname = gethostname()

    if is_manager:
        print(f"Starting manager {instancename} at {hostname}")
    else:
        print(f"Starting worker {instancename} at {hostname}")

    with DnetP2P("../../target/release/libdnet_p2p.dylib") as dnet:
        # dnet.enable_logs()  # enables rust logging
        dnet.create_instance(instancename, hostname, is_manager=is_manager)
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
