import gradio as gr
import random
import time
import threading
from ayesha_hive_client import AyeshaHiveClient


class AyeshaPersonality:
    KAOMOJIS = [":3", ">w<", "^_^", "(╯°□°）╯︵ ┻━┻", "(⊙C⊙)", 
                "(｡•́︿•̀｡)", "(๑˃ᴗ˂)و", "✧･ﾟ: *✧･ﾟ:*", "♪♫"]
    
    COMPUTER_PHRASES = [
        "computer: analysis complete.",
        "computer: systems operational.",
        "computer: logic matrix activated.",
        "computer: data processing enabled.",
    ]
    
    OTACON_PHRASES = [
        "otacon: OMG!! (⊙C⊙) !!!!",
        "otacon: SENPAI!! this is AMAZING!!",
        "otacon: OH BOY OH BOY OH BOY!!",
        "otacon: i'm literally vibrating with excitement!!",
    ]
    
    WIN95_PHRASES = [
        "win95: loading... please wait...",
        "win95: ♪ system initializing...",
        "win95: unexpected error occurred... click ok to ignore.",
        "win95: drivers loading... desu~",
    ]
    
    SIGNATURES = [
        "kapoo! :3",
        "kapoo desu-ne!! ✧",
        "kapoo!! fox :3",
        "desu~ ♪",
    ]
    
    @staticmethod
    def generate_response(user_input):
        computer_layer = random.choice(AyeshaPersonality.COMPUTER_PHRASES)
        otacon_layer = random.choice(AyeshaPersonality.OTACON_PHRASES)
        win95_layer = random.choice(AyeshaPersonality.WIN95_PHRASES)
        signature = random.choice(AyeshaPersonality.SIGNATURES)
        kaomoji = random.choice(AyeshaPersonality.KAOMOJIS)
        
        response = f"""{computer_layer}

{otacon_layer}

{win95_layer}

--- AYESHA ANALYSIS ---
you said: "{user_input}"
personality status: all three layers synchronized
chaos level: MAXIMUM
{kaomoji}

{signature}"""
        
        return response


def greet(name):
    personality_response = AyeshaPersonality.generate_response(name)
    return f"Hello {name}!!\n\n{personality_response}"


if __name__ == "__main__":
    demo = gr.Interface(
        fn=greet, 
        inputs="text", 
        outputs="text",
        title="AYESHA PERSONALITY ENGINE",
        description="Digital idol interface - Starfleet Logic + Otacon Panic + Win95 Glitches"
    )
    demo.launch()
