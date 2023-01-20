"""Basic GUI for timespoof"""
import socket
import tkinter as tk
from tkinter import ttk
import datetime
import time

class App(tk.Tk):
    """Basic GUI for timespoof"""
    TS = time.time()
    EPOCH = datetime.datetime(1601,1,1) \
        + (datetime.datetime.fromtimestamp(TS) \
        - datetime.datetime.utcfromtimestamp(TS))

    def __init__(self, *args, **kwargs) -> None:
        super().__init__(*args, **kwargs)
        self.connect_socket()
        self.initialize_widgets()

    def connect_socket(self):
        """Connect to libtimespoof via TCP socket"""
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.socket.bind(("127.0.0.1", 63463))
        self.socket.listen()
        self.conn, addr = self.socket.accept()
        print(f"Connected to {addr}")

    def initialize_widgets(self):
        """Draw GUI widgets"""
        self.date_entry_label = ttk.Label(self, text = "Date String (ISO 8601):")
        self.date_entry_label.grid(row = 0, column = 0, padx = 5, pady = 5)
        self.date_entry = ttk.Entry(self, width = 30)
        self.date_entry.insert(0, "2000-01-01 00:00:00.000000")
        self.date_entry.grid(row = 0, column = 1, padx = 5, pady = 5)
        self.real_time_bool = tk.BooleanVar()
        self.real_time_check = ttk.Checkbutton(
            self,
            text = "Real Time?",
            variable = self.real_time_bool
        )
        self.real_time_check.grid(row = 1, column = 0, columnspan = 2, padx = 5, pady = 5)
        self.move_forward_bool = tk.BooleanVar()
        self.move_forward_check = ttk.Checkbutton(
            self,
            text = "Move Forward?",
            variable = self.move_forward_bool
        )
        self.move_forward_check.grid(row = 2, column = 0, columnspan = 2, padx = 5, pady = 5)
        self.update_base_time_bool = tk.BooleanVar()
        self.update_base_time_check = ttk.Checkbutton(
            self,
            text = "Update Base Time?",
            variable = self.update_base_time_bool
        )
        self.update_base_time_check.grid(row = 3, column = 0, columnspan = 2, padx = 5, pady = 5)
        self.submit_button = ttk.Button(self, text = "Submit Settings", command = self.submit_time)
        self.submit_button.grid(
            row = 4,
            column = 0,
            columnspan = 2,
            padx = 5,
            pady = 5,
            sticky = "we"
        )

    def submit_time(self) -> None:
        """Send selected time to libtimespoof"""
        target = datetime.datetime.fromisoformat(self.date_entry.get())
        nano_div_100 = ((target - self.EPOCH) / datetime.timedelta(microseconds = 1)) * 10
        nano_div_100 = int(nano_div_100)
        real_time = int(self.real_time_bool.get())
        move_forward = int(self.move_forward_bool.get())
        update_base_time = int(self.update_base_time_bool.get())
        self.conn.send(
            f"{nano_div_100} {real_time} {move_forward} {update_base_time}".encode("utf-8")
        )
        print(self.conn.recv(1024).decode("utf-8"))

if __name__ == "__main__":
    app = App()
    app.mainloop()
