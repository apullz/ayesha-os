╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║     ██████╗ ██████╗ ███████╗███╗   ███╗██╗ ██████╗         ║
║    ██╔════╝██╔═══██╗██╔════╝████╗ ████║██║██╔════╝         ║
║    ██║     ██║   ██║███████╗██╔████╔██║██║██║              ║
║    ██║     ██║   ██║╚════██║██║╚██╔╝██║██║██║              ║
║    ╚██████╗╚██████╔╝███████║██║ ╚═╝ ██║██║╚██████╗         ║
║     ╚═════╝ ╚═════╝ ╚══════╝╚═╝     ╚═╝╚═╝ ╚═════╝         ║
║                                                              ║
║       ██████╗  █████╗  ██████╗                              ║
║       ██╔══██╗██╔══██╗██╔════╝                              ║
║       ██████╔╝███████║██║  ███╗                             ║
║       ██╔══██╗██╔══██║██║   ██║                             ║
║       ██║  ██║██║  ██║╚██████╔╝                             ║
║       ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝                              ║
║                                                              ║
║        cosmic-rag  ::  v1.0                                  ║
║        "local rag chatbot with vault knowledge base"         ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

  ── R E L E A S E   I N F O ──

  Title    : COSMIC-RAG
  Version  : v1.0
  Type     : RAG Chatbot / Knowledge Base
  Language : Python 3.10+
  Model    : llama3.2 (via local ollama)
  Status   : 100% LOCAL — no external APIs

  ── D E S C R I P T I O N ──

  cosmic-rag is a fully local Retrieval-Augmented Generation
  chatbot. it reads documents from a vault/ directory, chunks
  and embeds them, stores the vectors in-memory, and answers
  questions using the relevant context + a local ollama model.

  no embedding API, no vector database service, no external
  LLM. everything runs on your machine.

  ── H O W   I T   W O R K S ──

  1. documents in vault/ are loaded (txt, pdf, md)
  2. split into chunks of ~500 tokens with overlap
  3. embedded using ollama's nomic-embed-text
  4. stored in an in-memory FAISS index
  5. on query: embed the question, find top-k chunks
  6. send chunks + question to llama3.2 via ollama
  7. return context-aware answer

  ── I N S T A L L ──

  pip install ollama langchain faiss-cpu pypdf
  ollama pull llama3.2
  ollama pull nomic-embed-text

  ── U S A G E ──

  python main.py
  # drops into interactive chat with RAG context

  or use as a library:
  from cosmic_rag import ask
  answer = ask("what does the vault say about x?")

  ── F I L E S ──

  main.py                 entry point — interactive chat
  main.cosmic-rag.py      alternative entry
  vault/                  directory — drop documents here
  .gitignore              ignores vault contents

  ── N O T E S ──

  - faiss-cpu is used for vector similarity search
  - the vault/ directory is gitignored — add your own docs
  - embedding model: nomic-embed-text (1.8b, local)
  - generation model: llama3.2 (via ollama)

  ── G R E E T S ──

  langchain for the RAG plumbing. faiss for the vector search.
  ollama for keeping embeddings local.
  your documents are safe with us :3
