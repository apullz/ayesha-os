FROM ollama/ollama:latest

# install python + pip for the gradio app
RUN apt-get update && apt-get install -y --no-install-recommends python3 python3-pip curl \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --no-cache-dir --break-system-packages gradio requests

COPY Modelfile /root/Modelfile
COPY app.py /root/app.py
COPY ui.py theme.css chat.js overlay.html ayesha-bot.png /root/

# pull the base model and bake the ayesha model into the image at build time
RUN (nohup ollama serve > /tmp/ollama-build.log 2>&1 &) && sleep 6 \
    && ollama pull nemotron-3-nano:4b \
    && ollama create ayesha -f /root/Modelfile

ENV OLLAMA_KEEP_ALIVE=24h
EXPOSE 7860

ENTRYPOINT []
CMD ["/bin/bash", "-c", "(nohup ollama serve > /tmp/ollama.log 2>&1 &) && python3 /root/app.py"]
