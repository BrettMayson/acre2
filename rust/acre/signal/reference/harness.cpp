// Runs the original C++ terrain loader, antenna model and propagation models on a real
// terrain and writes inputs + outputs for tests/golden.rs. Built by generate.sh.
#include <cstdio>
#include <cstdint>
#include <memory>
#include <string>
#include <vector>

#include "wrp/landscape.hpp"
#include "map/map.hpp"
#include "antenna/antenna.hpp"
#include "models/los_simple.hpp"
#include "models/longleyRice.hpp"

using namespace acre::signal;

static uint32_t lcg_state = 12345;
static float rnd() { // [0, 1)
    lcg_state = lcg_state * 1664525u + 1013904223u;
    return (lcg_state >> 8) / 16777216.0f;
}

static void vec(FILE *f, const glm::vec3 &v) { fprintf(f, " %.9g %.9g %.9g", v.x, v.y, v.z); }

int main(int argc, char **argv) {
    if (argc != 5) {
        fprintf(stderr, "usage: harness <terrain.fakewrp> <whip.aba> <prc343.aba> <out>\n");
        return 1;
    }
    auto landscape = std::make_shared<acre::wrp::landscape>(std::string(argv[1]));
    auto map = std::make_shared<acre::signal::map>(landscape);
    map->setMapClimate(static_cast<MapClimate>(5));
    std::ifstream whip_file(argv[2], std::ios::binary), prc343_file(argv[3], std::ios::binary);
    auto whip = std::make_shared<antenna>(whip_file);
    auto prc343 = std::make_shared<antenna>(prc343_file);

    FILE *out = fopen(argv[4], "w");
    const float extent = map->map_size() * map->cell_size();
    fprintf(out, "map %u %.9g %d\n", map->map_size(), map->cell_size(), (int)landscape->failure);
    fprintf(out, "peaks %zu\n", map->peaks.size());
    for (auto &p : map->peaks) { fprintf(out, "peak"); vec(out, p); fprintf(out, "\n"); }

    for (int i = 0; i < 200; ++i) {
        const float x = rnd() * extent, y = rnd() * extent;
        float dx = 0, dy = 0;
        const float z = map->elevation(x, y, &dx, &dy);
        const glm::vec3 n = map->normal(x, y);
        fprintf(out, "elev %.9g %.9g %.9g %.9g %.9g", x, y, z, dx, dy); vec(out, n); fprintf(out, "\n");
    }

    for (int i = 0; i < 100; ++i) {
        const glm::vec3 a(rnd() * 2 - 1, rnd() * 2 - 1, rnd() * 2 - 1);
        const glm::vec3 s(rnd() * 2 - 1, rnd() * 2 - 1, rnd() * 2 - 1);
        const float f = 30.0f + rnd() * 480.0f;
        fprintf(out, "gain"); vec(out, a); vec(out, s);
        fprintf(out, " %.9g %.9g\n", f, whip->gain(a, s, f));
    }

    for (int i = 0; i < 120; ++i) {
        const bool uhf = (i % 3) == 2;
        const antenna_p ant = uhf ? prc343 : whip;
        const float f = uhf ? 2405.0f + rnd() * 70.0f : 40.0f + rnd() * 400.0f;
        const float power = uhf ? 100.0f : 5000.0f;
        const float margin = 1000.0f;
        const float tx_x = margin + rnd() * (extent - 2 * margin), tx_y = margin + rnd() * (extent - 2 * margin);
        const float angle = rnd() * 6.2831853f;
        const float dist = 100.0f + rnd() * (uhf ? 2000.0f : 9000.0f);
        const float rx_x = tx_x + std::cos(angle) * dist, rx_y = tx_y + std::sin(angle) * dist;
        const glm::vec3 tx(tx_x, tx_y, map->elevation(tx_x, tx_y) + 1.0f + rnd() * 10.0f);
        const glm::vec3 rx(rx_x, rx_y, map->elevation(rx_x, rx_y) + 1.0f + rnd() * 10.0f);
        // SQF passes the antenna's forward vector (vectorDir): roughly horizontal.
        const glm::vec3 tx_dir(rnd() * 2 - 1, rnd() * 2 - 1, rnd() * 0.2f - 0.1f);
        const glm::vec3 rx_dir(rnd() * 2 - 1, rnd() * 2 - 1, rnd() * 0.2f - 0.1f);
        const float scale = (i % 4 == 3) ? 0.5f : 1.0f;
        const bool omni = (i % 5) == 4;

        result los_r, mp_r, itm_r;
        model::los_simple los(map);
        los.process(&los_r, tx, tx_dir, rx, rx_dir, ant, ant, f, power, scale, omni);
        model::multipath mp(map); // fresh: the C++ cache kept growing across positions
        mp.process(&mp_r, tx, tx_dir, rx, rx_dir, ant, ant, f, power, scale, omni);
        model::longleyRice itm(map);
        itm.process(&itm_r, tx, tx_dir, rx, rx_dir, ant, ant, f, power, false, omni, true);

        fprintf(out, "case %d", uhf ? 1 : 0);
        vec(out, tx); vec(out, tx_dir); vec(out, rx); vec(out, rx_dir);
        fprintf(out, " %.9g %.9g %.9g %d", f, power, scale, omni ? 1 : 0);
        fprintf(out, " %.9g %.9g %.9g %.9g %.9g %.9g %zu\n",
                los_r.result_dbm, los_r.result_v, mp_r.result_dbm, mp_r.result_v,
                itm_r.result_dbm, itm_r.result_v, mp_r.reflect_points.size());
    }
    fclose(out);
    return 0;
}
