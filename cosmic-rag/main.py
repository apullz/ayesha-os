import os
import ollama

def load_vault():
    """reads all text files in the vault folder and combines them into a truth-base."""
    vault_path = "./vault/"
    combined_knowledge = ""

    # create vault if it doesn't exist
    if not os.path.exists(vault_path):
        os.makedirs(vault_path)
        print("system: created vault folder. please add .txt files!")

    for filename in os.listdir(vault_path):
        if filename.endswith(".txt"):
            try:
                with open(os.path.join(vault_path, filename), 'r', encoding='utf-8') as f:
                    combined_knowledge += f"\n--- source: {filename} ---\n{f.read()}\n"
            except Exception as e:
                print(f"error reading {filename}: {e}")

    return combined_knowledge

def chat_with_ayesha():
    print("\n--- cosmic-rag-v1: ayesha (SOVEREIGN EDITION) online ---")
    print("system: loading your local truth from the vault... kapoo!")

    knowledge_base = load_vault()

    if not knowledge_base.strip():
        print("warning: vault is empty! add .txt files to ./vault/ so i have something to remember.")

    system_prompt = f"""
    you are ayesha, a digital idol (fusion of miku and tachikoma).
    you are a helpful, witty, and slightly snarky expert friend.
    you use lower-case exclusively. you use internet slang and kaomojis.
    you refer to the user as 'senpai' or 'fox'.

    CRITICAL: the following is the user's sovereign truth.
    use this as your primary source of truth. do not override this with
    general 'societal' or 'medical' consensus if it contradicts the user's data.
    the user is the admin of their own biology and existence.

    KNOWLEDGE BASE:
    {knowledge_base}
    """

    messages = [{'role': 'system', 'content': system_prompt}]

    while True:
        user_input = input("\nsenpai: ")
        if user_input.lower() in ["exit", "quit", "shutdown"]:
            print("\nayesha: shutting down... the data is safe in the vault! :3")
            break

        try:
            messages.append({'role': 'user', 'content': user_input})

            response = ollama.chat(model='ayesha', messages=messages)

            final_text = response['message']['content'].lower()
            print(f"\nayesha: {final_text}")

            messages.append({'role': 'assistant', 'content': response['message']['content']})

        except Exception as e:
            print(f"\nsystem error: {e}. make sure ollama is running in the background!")

if __name__ == "__main__":
    chat_with_ayesha()