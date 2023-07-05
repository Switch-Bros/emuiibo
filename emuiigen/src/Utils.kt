package com.xortroll.emuiibo.emuiigen

import org.apache.commons.io.IOUtils
import java.net.URL
import java.nio.charset.StandardCharsets
import java.nio.channels.Channels
import java.text.Normalizer
import java.io.FileOutputStream

class Utils {
    companion object {
        fun reverseUShort(ush: UShort) : UShort {
            val byte1 = ush.toByte();
            val byte2 = (ush.toInt() ushr 8).toByte();

            return (
                ((byte2.toInt() and 0xFF)) or
                ((byte1.toInt() and 0xFF) shl  8)
            ).toUShort()
        }

        fun netDownloadBytes(url: String) : ByteArray {
            val strm = URL(url).openStream();
            return IOUtils.toByteArray(strm);
        }

        fun netDownloadString(url: String) : String {
            val strm = URL(url).openStream();
            return IOUtils.toString(strm, StandardCharsets.UTF_8);
        }

        fun netDownloadFile(url: String, path: String) {
            val url_v = URL(url);
            val ch = Channels.newChannel(url_v.openStream());
            val fos = FileOutputStream(path);
            fos.getChannel().transferFrom(ch, 0, Long.MAX_VALUE);
        }

        fun unaccentString(str: String) : String {
            return Normalizer.normalize(str, Normalizer.Form.NFD).replace("[^\\p{ASCII}]".toRegex(), "");
        }

        fun truncateString(str: String, len: Int) : String {
            if(str.length > len) {
                return str.substring(0, len);
            }
            else {
                return str;
            }
        }
    }
}