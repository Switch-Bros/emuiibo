package com.xortroll.emuiibo.emuiigen
import org.json.JSONObject
import org.json.JSONArray

class AmiiboAPI {
    companion object {
        val Url = "https://www.amiiboapi.com/api/amiibo/";

        fun readApi() : Map<String, List<AmiiboAPIEntry>>? {
            try {
                val raw_json = Utils.netDownloadString(Url);
                val json_data = JSONObject(raw_json);
                val json_entries = json_data.getJSONArray("amiibo");

                val entry_map = mutableMapOf<String, List<AmiiboAPIEntry>>();
                for(i in 0 until json_entries.length()) {
                    val entry = AmiiboAPIEntry.fromJson(json_entries.getJSONObject(i));
                    val series = entry_map.get(entry.series_name)?.toMutableList();
                    series?.let {
                        series.add(entry);
                        entry_map.put(entry.series_name, series);
                    }
                    ?: let {
                        entry_map.put(entry.series_name, listOf(entry));
                    }
                }
                return entry_map;
            }
            catch(ex: Exception) {
                System.out.println("Exception reading AmiiboAPI: " + ex.toString());
                return null;
            }
        }
    }
}