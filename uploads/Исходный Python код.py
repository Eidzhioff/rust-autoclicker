import time
import threading
import os
import ctypes
import pystray
from PIL import Image

# ================= НАСТРОЙКИ =================
# Виртуальные коды клавиш (Virtual-Key Codes) из Windows API
VK_XBUTTON1 = 0x05  # Боковая кнопка 1 (обычно "Назад")
VK_XBUTTON2 = 0x06  # Боковая кнопка 2 (обычно "Вперед")
VK_F12 = 0x7B       # Клавиша F12 для экстренного выхода

# Какую боковую кнопку слушать
TARGET_BUTTON = VK_XBUTTON2  

CLICK_DELAY = 0.04  # 40 миллисекунд
ICON_FILE = "icon.ico"
# =============================================

# Подключаем системную библиотеку Windows для управления вводом
user32 = ctypes.windll.user32

is_holding = False
running = True
tray_icon = None

def is_key_pressed(vk_code):
    """Проверяет, нажата ли клавиша/кнопка мыши в данный момент через Windows API"""
    # GetAsyncKeyState возвращает состояние клавиши. 
    # Если старший бит равен 1 (0x8000), значит клавиша нажата.
    return user32.GetAsyncKeyState(vk_code) & 0x8000 != 0

def click_mouse_left():
    """Имитирует клик левой кнопкой мыши через Windows API"""
    # 0x0002 = MOUSEEVENTF_LEFTDOWN (нажатие)
    # 0x0004 = MOUSEEVENTF_LEFTUP (отпускание)
    user32.mouse_event(0x0002, 0, 0, 0, 0)
    user32.mouse_event(0x0004, 0, 0, 0, 0)

def click_loop():
    """Поток автокликера"""
    while running:
        # Вместо слушателя событий, мы просто опрашиваем состояние кнопки в цикле
        if is_key_pressed(TARGET_BUTTON):
            click_mouse_left()
            time.sleep(CLICK_DELAY)
        else:
            time.sleep(0.01)

def on_exit_clicked(icon, item):
    """Обработчик нажатия 'Выйти' в меню трея"""
    global running
    running = False
    icon.stop()

def setup_tray():
    """Настройка и запуск иконки в трее"""
    global tray_icon
    
    if not os.path.exists(ICON_FILE):
        print(f"Ошибка: Файл '{ICON_FILE}' не найден!")
        input("Нажмите Enter для выхода...")
        return

    image = Image.open(ICON_FILE)
    
    menu = pystray.Menu(
        pystray.MenuItem("Автокликер (40мс)", None, enabled=False),
        pystray.Menu.SEPARATOR,
        pystray.MenuItem("Выйти", on_exit_clicked)
    )
    
    tray_icon = pystray.Icon("autoclicker", image, "Автокликер активен", menu)
    tray_icon.run()

# --- ЗАПУСК ПРОГРАММЫ ---

# 1. Запуск потока кликов
click_thread = threading.Thread(target=click_loop, daemon=True)
click_thread.start()

# 2. Запуск иконки в трее (блокирует главный поток)
setup_tray()
