"""RIG

Usage:
    rig.py <paths>...
"""
import logging
import os
from typing import List

from docopt import docopt
from kivy import Config
from kivy.app import App
from kivy.core.window import Window
from kivy.properties import NumericProperty, ListProperty
from kivy.uix.floatlayout import FloatLayout
from kivy.uix.image import Image
from kivy.uix.label import Label
import filetype


class Rig(App):
    index = NumericProperty()
    images = ListProperty()

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.image = None
        self._keyboard = None
        self.label = "None"

    def build(self):
        self.image = Image(allow_stretch=True)
        self.label = Label(outline_width=2, pos=(0, 0), size_hint=(0.1, 0.1))
        self.reload_image()

        self._keyboard = Window.request_keyboard(self._keyboard_close, self.image)
        self._keyboard.bind(on_key_up=self.key_up)

        Config.set("input", "mouse", "mouse,multitouch_on_demand")
        Window.bind(on_touch_down=self.on_touch_down)

        layout = FloatLayout()
        layout.add_widget(self.image)
        layout.add_widget(self.label)
        return layout

    def reload_image(self):
        path = self.images[self.index]
        self.image.source = path
        self.label.text = os.path.basename(path)

    def next(self):
        self.index = 0 if self.index == len(self.images) - 1 else self.index + 1
        self.reload_image()

    def prev(self):
        self.index = len(self.images) - 1 if self.index == 0 else self.index - 1
        self.reload_image()

    def _keyboard_close(self, *args):
        if self._keyboard:
            self._keyboard.unbind(on_key_up=self.key_up)
            self._keyboard = None

    def key_up(self, keyboard, keycode, *args):
        # system keyboard keycode: (122, 'z')
        # dock keyboard keycode: 'z'
        if isinstance(keycode, tuple):
            keycode = keycode[1]

        if keycode == "f":
            Window.fullscreen = False if Window.fullscreen else "auto"
        if keycode == "spacebar" or keycode == "right":
            self.next()
        if keycode == "left":
            self.prev()

    def on_touch_down(self, window, touch):
        if hasattr(touch, "button"):
            if touch.button == "right":
                self.next()
            elif touch.button == "left":
                self.prev()
        return True


def find_images(paths: List[str]):
    for path in paths:
        if os.path.isdir(path):
            for root, _, files in os.walk(path, topdown=True):
                for name in files:
                    path = os.path.abspath(os.path.join(root, name))
                    try:
                        if filetype.is_image(path):
                            yield path
                    except Exception as e:
                        logging.warning("Failed to read file %s: %s" % (path, e))
        else:
            if filetype.is_image(path):
                yield path


if __name__ == "__main__":
    arguments = docopt(__doc__, version="vRIG 0.1")
    images = list(find_images(arguments["<paths>"]))
    logging.info("Found %s images" % len(images))
    Rig(images=images).run()
