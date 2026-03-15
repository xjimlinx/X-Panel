#!/usr/bin/env python3
"""
GPX 通知程序

支持两种通知方式：
1. TCP/IP 消息传递（同一网络内）
2. GNOME 桌面通知

使用方法：
    # 作为服务器运行（接收通知）
    python notify.py server
    
    # 发送 TCP 通知
    python notify.py send --host <IP> --port <PORT> --message "消息内容"
    
    # 发送 GNOME 通知
    python notify.py gnome --title "标题" --message "消息内容"
    
    # 发送任务完成通知
    python notify.py task --name "任务名"
"""

import argparse
import socket
import subprocess
import sys
import threading
import json
from datetime import datetime


class TaskNotifier:
    """任务完成通知"""
    
    @staticmethod
    def notify(task_name: str, message: str = None):
        """发送任务完成通知"""
        if message is None:
            message = f"X-Panel: 任务{task_name}已完成，请查阅。"
        
        print(f"[任务通知] {message}")
        TCPNotifier.send_gnome_notification("X-Panel", message)
        return True


class TCPNotifier:
    """TCP 通知服务端/客户端"""

    DEFAULT_PORT = 9527
    BUFFER_SIZE = 4096

    def __init__(self, host='0.0.0.0', port=None):
        self.host = host
        self.port = port or self.DEFAULT_PORT
        self.server_socket = None
        self.running = False

    def start_server(self):
        """启动 TCP 通知服务器"""
        self.server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.server_socket.bind((self.host, self.port))
        self.server_socket.listen(5)
        self.running = True

        print(f"[通知服务器] 监听中：{self.host}:{self.port}")

        while self.running:
            try:
                client_socket, addr = self.server_socket.accept()
                threading.Thread(
                    target=self.handle_client,
                    args=(client_socket, addr),
                    daemon=True
                ).start()
            except Exception as e:
                if self.running:
                    print(f"[错误] 接受连接失败：{e}")

    def handle_client(self, client_socket, addr):
        """处理客户端连接"""
        try:
            data = client_socket.recv(self.BUFFER_SIZE)
            if data:
                message = data.decode('utf-8')
                timestamp = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
                print(f"\n[{timestamp}] 收到来自 {addr[0]}:{addr[1]} 的通知:")
                print(f"  {message}")

                # 尝试发送 GNOME 通知
                self.send_gnome_notification("GPX 通知", message)

                client_socket.send(b"OK")
        except Exception as e:
            print(f"[错误] 处理客户端失败：{e}")
        finally:
            client_socket.close()

    def stop_server(self):
        """停止服务器"""
        self.running = False
        if self.server_socket:
            self.server_socket.close()

    def send_message(self, host, port, message):
        """发送 TCP 通知消息"""
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(5)
            sock.connect((host, port))
            sock.send(message.encode('utf-8'))
            response = sock.recv(self.BUFFER_SIZE)
            sock.close()
            print(f"[发送成功] 消息已发送至 {host}:{port}")
            print(f"[响应] {response.decode('utf-8')}")
            return True
        except Exception as e:
            print(f"[发送失败] {e}")
            return False

    @staticmethod
    def send_gnome_notification(title, message, urgency="normal"):
        """发送 GNOME 桌面通知"""
        try:
            # 检查是否支持 notify-send
            result = subprocess.run(
                ["which", "notify-send"],
                capture_output=True,
                text=True
            )
            if result.returncode != 0:
                print("[警告] notify-send 不可用，跳过 GNOME 通知")
                return False

            # 设置 urgency
            urgency_flag = "-u"
            subprocess.run([
                "notify-send",
                urgency_flag, urgency,
                "-a", "GPX",
                title,
                message
            ], check=True)
            print(f"[GNOME 通知] {title}: {message}")
            return True
        except subprocess.CalledProcessError as e:
            print(f"[GNOME 通知失败] {e}")
            return False
        except FileNotFoundError:
            print("[GNOME 通知失败] notify-send 未找到")
            return False


def cmd_server(args):
    """服务器命令"""
    notifier = TCPNotifier(host=args.host, port=args.port)
    try:
        notifier.start_server()
    except KeyboardInterrupt:
        print("\n[信息] 服务器已停止")
        notifier.stop_server()


def cmd_send(args):
    """发送命令"""
    notifier = TCPNotifier()
    notifier.send_message(args.host, args.port, args.message)


def cmd_gnome(args):
    """GNOME 通知命令"""
    TCPNotifier.send_gnome_notification(args.title, args.message, args.urgency)


def cmd_notify_me(args):
    """发送决定请求通知"""
    """当需要用户确认时调用此函数"""
    title = "GPX - 需要您的确认"
    message = f"{args.context}\n\n请您查看项目并作出决定。"

    if args.tcp_host:
        # 发送到 TCP 服务器
        notifier = TCPNotifier()
        notifier.send_message(args.tcp_host, args.tcp_port, message)
    else:
        # 发送 GNOME 通知
        TCPNotifier.send_gnome_notification(title, message)


def cmd_task(args):
    """发送任务完成通知"""
    TaskNotifier.notify(args.name, args.message)


def main():
    parser = argparse.ArgumentParser(
        description="GPX 通知程序 - 支持 TCP 和 GNOME 通知"
    )
    subparsers = parser.add_subparsers(dest='command', help='命令')

    # server 命令
    server_parser = subparsers.add_parser('server', help='启动通知服务器')
    server_parser.add_argument('--host', default='0.0.0.0', help='监听地址')
    server_parser.add_argument('--port', type=int, default=9527, help='监听端口')
    server_parser.set_defaults(func=cmd_server)

    # send 命令
    send_parser = subparsers.add_parser('send', help='发送 TCP 通知')
    send_parser.add_argument('--host', required=True, help='目标主机 IP')
    send_parser.add_argument('--port', type=int, default=9527, help='目标端口')
    send_parser.add_argument('--message', required=True, help='消息内容')
    send_parser.set_defaults(func=cmd_send)

    # gnome 命令
    gnome_parser = subparsers.add_parser('gnome', help='发送 GNOME 通知')
    gnome_parser.add_argument('--title', default='GPX 通知', help='通知标题')
    gnome_parser.add_argument('--message', required=True, help='消息内容')
    gnome_parser.add_argument(
        '--urgency',
        choices=['low', 'normal', 'critical'],
        default='normal',
        help='通知紧急程度'
    )
    gnome_parser.set_defaults(func=cmd_gnome)

    # notify-me 命令（用于项目决策通知）
    notify_me_parser = subparsers.add_parser('notify-me', help='发送决策请求通知')
    notify_me_parser.add_argument('--context', required=True, help='需要决策的上下文')
    notify_me_parser.add_argument('--tcp-host', help='TCP 服务器主机（可选）')
    notify_me_parser.add_argument('--tcp-port', type=int, default=9527, help='TCP 服务器端口')
    notify_me_parser.set_defaults(func=cmd_notify_me)

    # task 命令（用于任务完成通知）
    task_parser = subparsers.add_parser('task', help='发送任务完成通知')
    task_parser.add_argument('--name', required=True, help='任务名称')
    task_parser.add_argument('--message', help='自定义消息（可选）')
    task_parser.set_defaults(func=cmd_task)

    args = parser.parse_args()

    if args.command is None:
        parser.print_help()
        sys.exit(1)

    args.func(args)


if __name__ == '__main__':
    main()
