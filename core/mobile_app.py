import kivy
from kivy.app import App
from kivy.uix.boxlayout import BoxLayout
from kivy.uix.label import Label
from kivy.uix.textinput import TextInput
from kivy.uix.button import Button
from kivy.uix.scrollview import ScrollView

class ayesha_brain:
    def __init__(self):
        self.modes = ['computer', 'otacon', 'win95']
        self.current_mode = 0

    def generate_response(self, text):
        mode = self.modes[self.current_mode]
        self.current_mode = (self.current_mode + 1) % 3
        if mode == 'computer':
            return f'[computer]: analysis of {text} complete. result: nominal, desu.'
        elif mode == 'otacon':
            return f'[otacon]: senpai!! {text}?! this is unbelievable!! (⊙C⊙)'
        else:
            return f'[win95]: error 0x800... {text} not found in registry. (｡•́︿•̀｡)'

class ayesha_mobile(App):
    def build(self):
        self.title = 'ayesha_core_v1.0'
        self.brain = ayesha_brain()
        self.layout = BoxLayout(orientation='vertical', padding=10, spacing=10)
        self.scroll = ScrollView(size_hint=(1, 0.8))
        self.log = Label(text='[system] welcome to ayesha-os...\n', size_hint_y=None, halign='left', valign='top')
        self.log.bind(size=self.update_text_width)
        self.scroll.add_widget(self.log)
        self.input_area = BoxLayout(size_hint=(1, 0.2), spacing=10)
        self.user_input = TextInput(multiline=False, hint_text='talk to ayesha...', font_size=18)
        self.send_btn = Button(text='SEND', size_hint_x=0.3)
        self.send_btn.bind(on_press=self.send_message)
        self.input_area.add_widget(self.user_input)
        self.input_area.add_widget(self.send_btn)
        self.layout.add_widget(self.scroll)
        self.layout.add_widget(self.input_area)
        return self.layout

    def update_text_width(self, instance, value):
        self.log.text_size = (instance.width, None)
        self.log.height = self.log.texture_size[1]

    def send_message(self, instance):
        text = self.user_input.text
        if text:
            self.log.text += f'\n\nuser: {text}\n{self.brain.generate_response(text)}'
            self.user_input.text = ''
            self.scroll.scroll_y = 0

if __name__ == '__main__':
    ayesha_mobile().run()
