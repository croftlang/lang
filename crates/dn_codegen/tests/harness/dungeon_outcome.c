/* Runs the Dungeon for the test target, then prints what the repositories
   hold, so the test can check the game played out as scripted. */
#include <stdio.h>
#include "program.h"

extern Dungeon_Room Dungeon_Rooms_items[];
extern Scoring_Scoreboard Scoring_Scoreboards_items[];
int dn_program_main(int argc, char **argv);

int main(int argc, char **argv)
{
    int status = dn_program_main(argc, argv);
    Dungeon_Room *room = &Dungeon_Rooms_items[0];
    printf("status %d\n", status);
    printf("player %ld,%ld\n", (long) room->player.position.x, (long) room->player.position.y);
    printf("enemies %u treasure %u\n", (unsigned) room->enemies.length, (unsigned) room->treasure.length);
    printf("points %ld\n", (long) Scoring_Scoreboards_items[0].points);
    return 0;
}
