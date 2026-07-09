package com.companion.ayeshaterminal

import android.net.Uri
import android.os.Bundle
import android.util.Base64
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.InputStream
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL

// --- Android Oh My Posh / Windows Terminal Style Colors ---
val BgColor = Color(0xFF0C0F17)
val HeaderBg = Color(0xFF141824)
val AccentPurple = Color(0xFFA855F7)
val AccentBlue = Color(0xFF3B82F6)
val AccentEmerald = Color(0xFF10B981)
val TextWhite = Color(0xFFE2E8F0)
val TextGray = Color(0xFF64748B)

data class ConsoleMessage(
    val id: String,
    val sender: String, // "system", "user", "bot"
    val text: String,
    val timestamp: String
)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = BgColor
                ) {
                    TerminalMainScreen()
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TerminalMainScreen() {
    val context = LocalContext.current
    
    // Configurations: Customize to point to your PC's local Ollama endpoint over Wi-Fi
    // Keep in mind to configure OLLAMA_HOST="0.0.0.0" on your PC!
    var ollamaHost by remember { mutableStateOf("http://192.168.1.50:11434") }
    var inputPrompt by remember { mutableStateOf("") }
    
    // Image selection state parameters
    var imageUri by remember { mutableStateOf<Uri?>(null) }
    var imageBase64 by remember { mutableStateOf<String?>(null) }
    var attachedFileName by remember { mutableStateOf("") }

    val imagePickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.GetContent()
    ) { uri: Uri? ->
        if (uri != null) {
            imageUri = uri
            attachedFileName = uri.lastPathSegment ?: "image.jpg"
            
            // Read binary inputstream to Base64 in standard thread
            CoroutineScope(Dispatchers.IO).launch {
                try {
                    val stream: InputStream? = context.contentResolver.openInputStream(uri)
                    val bytes = stream?.readBytes()
                    if (bytes != null) {
                        imageBase64 = Base64.encodeToString(bytes, Base64.NO_WRAP)
                    }
                } catch (e: Exception) {
                    // Fallback
                }
            }
        }
    }

    val consoleLogs = remember { 
        mutableStateListOf<ConsoleMessage>().apply {
            add(
                ConsoleMessage(
                    id = "init-hdr",
                    sender = "system",
                    text = "Ayesha Companion Terminal [Version 4.5 Android]\n(c) 2026 Ayesha Corp. All rights reserved.\n\n✨ Oh My Posh container 'Android-Hacker' loaded over local sockets.\n📎 Local Gallery Photo Upload Option Activated!",
                    timestamp = ""
                )
            )
            add(
                ConsoleMessage(
                    id = "init-greeting",
                    sender = "bot",
                    text = "(╯°□°)╯︵ ┻━┻ Omg senpaii! I am ready to analyze photos and screens locally!\n\n💡 Tap '📷 UPLOAD' on the prompt line to select any photo from your gallery. Type a query and hit Send to analyze it offline with Moondream VLM!",
                    timestamp = ""
                )
            )
        }
    }

    var isGenerating by remember { mutableStateOf(false) }

    Column(modifier = Modifier.fillMaxSize()) {
        // 1. Android Title bar / Target setting
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .background(HeaderBg)
                .padding(horizontal = 16.dp, vertical = 12.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "📟 pwsh: local_companion",
                    color = Color.White,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    fontSize = 13.sp
                )
                
                Surface(
                    color = Color(0x3310B981),
                    shape = RoundedCornerShape(4.dp)
                ) {
                    Text(
                        text = "WI-FI ACTIVE",
                        color = AccentEmerald,
                        fontSize = 9.sp,
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                    )
                }
            }
            
            Spacer(modifier = Modifier.height(6.dp))
            
            // Ollama Wi-Fi target input field
            OutlinedTextField(
                value = ollamaHost,
                onValueChange = { ollamaHost = it },
                label = { Text("PC Local Wi-Fi Ollama Endpoint URL", color = TextGray, fontSize = 10.sp) },
                colors = TextFieldDefaults.outlinedTextFieldColors(
                    focusedBorderColor = AccentPurple,
                    unfocusedBorderColor = TextGray,
                    textColor = Color.White
                ),
                textStyle = LocalTextStyle.current.copy(fontFamily = FontFamily.Monospace, fontSize = 11.sp),
                singleLine = true,
                modifier = Modifier.fillMaxWidth()
            )
        }

        // 2. Terminal outputs scroll view
        LazyColumn(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            contentPadding = PaddingValues(top = 16.dp, bottom = 16.dp)
        ) {
            items(consoleLogs, key = { it.id }) { msg ->
                when (msg.sender) {
                    "system" -> {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .background(Color(0xFF141824), RoundedCornerShape(6.dp))
                                .padding(10.dp)
                        ) {
                            Text(
                                text = msg.text,
                                color = Color(0xFF38BDF8),
                                fontFamily = FontFamily.Monospace,
                                fontSize = 11.sp
                            )
                        }
                    }
                    "user" -> {
                        // Mimics beautiful Oh My Posh Segment
                        Column {
                            Row(
                                modifier = Modifier.height(20.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                // apullz tag
                                Box(
                                    modifier = Modifier
                                        .fillMaxHeight()
                                        .background(AccentPurple)
                                        .padding(horizontal = 8.dp),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text("⚡ apullz", color = Color.Black, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                                }
                                // Directory tag
                                Box(
                                    modifier = Modifier
                                        .fillMaxHeight()
                                        .background(AccentBlue)
                                        .padding(horizontal = 8.dp),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text("~", color = Color.White, fontSize = 10.sp)
                                }
                            }
                            Spacer(modifier = Modifier.height(4.dp))
                            Text(
                                text = "$ ${msg.text}",
                                color = TextWhite,
                                fontFamily = FontFamily.Monospace,
                                fontSize = 13.sp,
                                modifier = Modifier.padding(start = 6.dp)
                            )
                        }
                    }
                    "bot" -> {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(start = 8.dp)
                        ) {
                            Row(
                                modifier = Modifier.height(20.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Box(
                                    modifier = Modifier
                                        .fillMaxHeight()
                                        .background(Color(0xFFFF5722))
                                        .padding(horizontal = 8.dp),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text("👾 ayesha-bot", color = Color.White, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                                }
                            }
                            Spacer(modifier = Modifier.height(4.dp))
                            Text(
                                text = msg.text,
                                color = TextWhite,
                                fontFamily = FontFamily.Monospace,
                                fontSize = 13.sp,
                                modifier = Modifier.padding(start = 6.dp)
                            )
                        }
                    }
                }
            }

            if (isGenerating) {
                item {
                    Text(
                        text = "● [Transmitting to local PC Ollama VLM node...]",
                        color = AccentPurple,
                        fontFamily = FontFamily.Monospace,
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold
                    )
                }
            }
        }

        // 3. Optional visual file attachment status bar (mimicker)
        if (imageBase64 != null) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .background(Color(0xFF141824))
                    .padding(horizontal = 16.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = "📎 Attached Image: ",
                        color = AccentPurple,
                        fontSize = 11.sp,
                        fontFamily = FontFamily.Monospace,
                        fontWeight = FontWeight.Bold
                    )
                    Text(
                        text = attachedFileName,
                        color = Color.White,
                        fontSize = 11.sp,
                        fontFamily = FontFamily.Monospace
                    )
                }
                Text(
                    text = "[X] Detach",
                    color = Color.Red,
                    fontSize = 11.sp,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.clickable {
                        imageBase64 = null
                        imageUri = null
                        attachedFileName = ""
                    }
                )
            }
        }

        // 4. Command Line Interface Input Box (No redundant button noise)
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(Color(0xFF07090E))
                .padding(12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Pre-cursor styling segment
            Box(
                modifier = Modifier
                    .background(AccentPurple, RoundedCornerShape(2.dp))
                    .padding(horizontal = 6.dp, vertical = 2.dp)
            ) {
                Text("⚡ android", color = Color.Black, fontSize = 9.sp, fontWeight = FontWeight.Bold)
            }
            Text(" >", color = AccentPurple, fontWeight = FontWeight.Bold)
            
            Spacer(modifier = Modifier.width(8.dp))

            // Attachment button triggers gallery
            Surface(
                color = if (imageBase64 != null) AccentEmerald.copy(alpha = 0.2f) else HeaderBg,
                shape = RoundedCornerShape(4.dp),
                modifier = Modifier
                    .clickable { imagePickerLauncher.launch("image/*") }
                    .padding(end = 8.dp)
            ) {
                Text(
                    text = if (imageBase64 != null) "📎 READY" else "📷 UPLOAD",
                    color = if (imageBase64 != null) AccentEmerald else Color.White,
                    fontFamily = FontFamily.Monospace,
                    fontSize = 9.sp,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 4.dp)
                )
            }

            BasicTextField(
                value = inputPrompt,
                onValueChange = { inputPrompt = it },
                textStyle = androidx.compose.ui.text.TextStyle(
                    color = Color.White,
                    fontFamily = FontFamily.Monospace,
                    fontSize = 14.sp
                ),
                keyboardOptions = KeyboardOptions(
                    imeAction = ImeAction.Send
                ),
                keyboardActions = KeyboardActions(
                    onSend = {
                        val trimmed = inputPrompt.trim()
                        val hasAttachedImage = imageBase64 != null
                        val currentImageB64 = imageBase64
                        
                        // Prevent loops: Clear attachments instantly after capture
                        imageBase64 = null
                        imageUri = null
                        attachedFileName = ""

                        if ((trimmed.isNotEmpty() || hasAttachedImage) && !isGenerating) {
                            val submitPrompt = trimmed.ifEmpty { "Describe this image in detail." }
                            consoleLogs.add(
                                ConsoleMessage(
                                    id = System.currentTimeMillis().toString(),
                                    sender = "user",
                                    text = if (hasAttachedImage) "📎 [Sent Photo] $submitPrompt" else submitPrompt,
                                    timestamp = ""
                                )
                            )
                            isGenerating = true
                            inputPrompt = ""
                            
                            // Send query on background thread
                            CoroutineScope(Dispatchers.IO).launch {
                                val reply = fetchOllamaReply(ollamaHost, submitPrompt, currentImageB64)
                                withContext(Dispatchers.Main) {
                                    consoleLogs.add(
                                        ConsoleMessage(
                                            id = System.currentTimeMillis().toString(),
                                            sender = "bot",
                                            text = reply,
                                            timestamp = ""
                                        )
                                    )
                                    isGenerating = false
                                }
                            }
                        }
                    }
                ),
                modifier = Modifier.weight(1f)
            )
        }
    }
}

suspend fun fetchOllamaReply(host: String, prompt: String, imageB64: String?): String {
    return try {
        // Direct HTTP POST to Ollama Core Generate API
        val targetUrl = URL("$host/api/generate")
        val conn = targetUrl.openConnection() as HttpURLConnection
        conn.requestMethod = "POST"
        conn.setRequestProperty("Content-Type", "application/json")
        conn.doOutput = true
        conn.readTimeout = 50000
        conn.connectTimeout = 5000

        val payload = JSONObject().apply {
            put("model", if (imageB64 != null) "moondream:latest" else "ayesha:latest")
            put("prompt", prompt)
            put("stream", false)
            if (imageB64 != null) {
                val array = org.json.JSONArray().apply {
                    put(imageB64)
                }
                put("images", array)
            }
        }

        OutputStreamWriter(conn.outputStream).use { writer ->
            writer.write(payload.toString())
        }

        if (conn.responseCode == 200) {
            val responseString = conn.inputStream.bufferedReader().use { it.readText() }
            val responseJson = JSONObject(responseString)
            responseJson.getString("response")
        } else {
            "⚠️ Request Error: HTTP Response code " + conn.responseCode
        }
    } catch (e: Exception) {
        "⚠️ Connection Failed! Make sure OLLAMA_HOST is 0.0.0.0 and correct local Wi-Fi port host is given.\nErr details: " + e.localizedMessage
    }
}
