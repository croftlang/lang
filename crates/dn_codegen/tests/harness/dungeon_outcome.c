/* Runs the Dungeon for the test target, then finds the first room and the
   main scoreboard, so the test can check the game played out as scripted. */
#include <stdio.h>
#include "program.h"

int dn_program_main(int argc, char **argv);

int main(int argc, char **argv)
{
    int status = dn_program_main(argc, argv);
    dn_result_Dungeon_Room_error room;
    dn_result_Scoring_Scoreboard_error board;
    Dungeon_Rooms_find(1, &room);
    Scoring_Scoreboards_find(1, &board);
    printf("status %d\n", status);
    printf("player %ld,%ld\n", (long) room.value.player.position.x,
           (long) room.value.player.position.y);
    printf("enemies %u treasure %u\n", (unsigned) room.value.enemies.length,
           (unsigned) room.value.treasure.length);
    printf("points %ld\n", (long) board.value.points);
    return 0;
}
